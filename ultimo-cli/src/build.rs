//! `ultimo build` — a production build of an Ultimo project.
//!
//! Works for both scaffold layouts:
//!
//! - **single crate** (`basic`, `api-only`, `rpc`, `production`): `Cargo.toml`
//!   at the project root;
//! - **fullstack**: a Rust crate in `backend/` and a JavaScript app in
//!   `frontend/`.
//!
//! The steps run in dependency order, and the first failure stops the build:
//!
//! 1. compile the Rust crate with the chosen profile;
//! 2. regenerate the typed TypeScript client (when there is a frontend, or
//!    `--client` asks for it) so the frontend never builds against a stale
//!    client — reusing the binaries step 1 just built;
//! 3. build the frontend with its own package manager.

use anyhow::{bail, Context, Result};
use colored::Colorize;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::generate;

/// The binary `ultimo generate` runs to emit the TypeScript client.
const GENERATE_CLIENT_BIN: &str = "generate-client";

/// Cargo build profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    Debug,
    Release,
}

impl Profile {
    /// Parse the `--profile` value (clap already restricts it to these).
    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "release" => Ok(Profile::Release),
            "debug" => Ok(Profile::Debug),
            other => bail!("unknown profile `{other}`: expected `debug` or `release`"),
        }
    }

    fn is_release(self) -> bool {
        self == Profile::Release
    }

    /// The directory under cargo's target dir that holds this profile's output.
    fn target_subdir(self) -> &'static str {
        match self {
            Profile::Debug => "debug",
            Profile::Release => "release",
        }
    }

    fn cargo_command(self) -> &'static str {
        match self {
            Profile::Debug => "cargo build",
            Profile::Release => "cargo build --release",
        }
    }
}

/// Options for `ultimo build`, straight from the command line.
pub struct Options {
    pub project: PathBuf,
    pub profile: Profile,
    pub client: Option<PathBuf>,
    pub skip_client: bool,
    pub skip_frontend: bool,
}

/// Where the parts of a project live.
#[derive(Debug, PartialEq, Eq)]
pub struct Layout {
    /// Directory holding the Rust crate's `Cargo.toml`.
    pub backend: PathBuf,
    /// Directory holding the frontend's `package.json`, if there is one.
    pub frontend: Option<PathBuf>,
}

/// Work out the project layout from its root directory.
pub fn detect_layout(root: &Path) -> Result<Layout> {
    let backend = if root.join("backend/Cargo.toml").is_file() {
        root.join("backend")
    } else if root.join("Cargo.toml").is_file() {
        root.to_path_buf()
    } else {
        bail!(
            "No Cargo.toml found in {root} or {root}/backend.\n\
             Run `ultimo build` from your project's root directory, or point at it \
             with `--project <dir>`.",
            root = root.display()
        );
    };

    let frontend = root.join("frontend");
    let frontend = frontend.join("package.json").is_file().then_some(frontend);

    Ok(Layout { backend, frontend })
}

/// Pick the package manager a frontend uses, from its lockfile.
pub fn package_manager(frontend: &Path) -> &'static str {
    if frontend.join("pnpm-lock.yaml").is_file() {
        "pnpm"
    } else if frontend.join("yarn.lock").is_file() {
        "yarn"
    } else if frontend.join("bun.lockb").is_file() || frontend.join("bun.lock").is_file() {
        "bun"
    } else {
        "npm"
    }
}

pub async fn run(opts: Options) -> Result<()> {
    let root = absolutize(&opts.project)?;
    let layout = detect_layout(&root)?;
    let profile = opts.profile;

    println!(
        "{}",
        format!(
            "📦 Building {} ({})",
            root.display(),
            profile.target_subdir()
        )
        .bold()
    );

    // 1. Rust
    step("Compiling the Rust crate", profile.cargo_command());
    let mut cargo = Command::new("cargo");
    cargo.arg("build").current_dir(&layout.backend);
    if profile.is_release() {
        cargo.arg("--release");
    }
    let status = cargo
        .status()
        .context("Failed to run `cargo`. Is the Rust toolchain installed and on your PATH?")?;
    if !status.success() {
        bail!(
            "`{}` failed in {}. Fix the errors above, then run `ultimo build` again.",
            profile.cargo_command(),
            layout.backend.display()
        );
    }

    let meta = CargoMetadata::read(&layout.backend)?;

    // 2. TypeScript client
    let client = if opts.skip_client {
        None
    } else {
        opts.client
            .as_deref()
            .map(absolutize)
            .transpose()?
            .or_else(|| {
                layout
                    .frontend
                    .as_ref()
                    .map(|f| f.join("src/generated/client.ts"))
            })
    };
    match client {
        Some(output) if meta.has_bin(GENERATE_CLIENT_BIN) => {
            step(
                "Regenerating the TypeScript client",
                &format!("cargo run --bin {GENERATE_CLIENT_BIN}"),
            );
            generate::generate_once(&layout.backend, &output, profile.is_release())?;
        }
        // Asked for explicitly, so a missing generator is an error.
        Some(_) if opts.client.is_some() => bail!(
            "`--client` was given, but {} has no `{GENERATE_CLIENT_BIN}` binary.\n\
             Add `src/bin/{GENERATE_CLIENT_BIN}.rs` (see `ultimo generate --help`), \
             or drop `--client`.",
            layout.backend.display()
        ),
        Some(_) => skipped(&format!(
            "TypeScript client: no `{GENERATE_CLIENT_BIN}` binary in {}",
            layout.backend.display()
        )),
        None => {}
    }

    // 3. Frontend
    let mut frontend_out = None;
    match &layout.frontend {
        Some(_) if opts.skip_frontend => skipped("frontend: --skip-frontend"),
        Some(frontend) => frontend_out = build_frontend(frontend)?,
        None => {}
    }

    // Summary
    println!();
    println!("{}", "✨ Build complete".green().bold());
    for bin in meta.bins.iter().filter(|b| *b != GENERATE_CLIENT_BIN) {
        let path = meta
            .target_directory
            .join(profile.target_subdir())
            .join(format!("{bin}{}", std::env::consts::EXE_SUFFIX));
        if path.is_file() {
            println!("   🦀 {}", path.display().to_string().cyan());
        }
    }
    if let Some(dist) = frontend_out {
        println!("   🌐 {}", dist.display().to_string().cyan());
    }

    Ok(())
}

/// Install dependencies if needed, then run the `build` script. Returns the
/// output directory when it can be found.
fn build_frontend(frontend: &Path) -> Result<Option<PathBuf>> {
    let manifest_path = frontend.join("package.json");
    let manifest: Value = serde_json::from_str(
        &std::fs::read_to_string(&manifest_path)
            .with_context(|| format!("Failed to read {}", manifest_path.display()))?,
    )
    .with_context(|| format!("{} is not valid JSON", manifest_path.display()))?;

    if manifest.pointer("/scripts/build").is_none() {
        skipped(&format!(
            "frontend: {} has no `build` script",
            manifest_path.display()
        ));
        return Ok(None);
    }

    let pm = package_manager(frontend);
    let has_deps = ["dependencies", "devDependencies"].iter().any(|key| {
        manifest
            .get(key)
            .and_then(Value::as_object)
            .is_some_and(|deps| !deps.is_empty())
    });
    if has_deps && !frontend.join("node_modules").is_dir() {
        step("Installing frontend dependencies", &format!("{pm} install"));
        run_js(pm, &["install"], frontend)?;
    }

    step("Building the frontend", &format!("{pm} run build"));
    run_js(pm, &["run", "build"], frontend)?;

    let dist = frontend.join("dist");
    Ok(dist.is_dir().then_some(dist))
}

fn run_js(pm: &str, args: &[&str], dir: &Path) -> Result<()> {
    // On Windows the package managers are `.cmd` shims, which `Command` won't
    // find under their bare name.
    let program = if cfg!(windows) {
        format!("{pm}.cmd")
    } else {
        pm.to_string()
    };
    let shown = format!("{pm} {}", args.join(" "));

    let status = match Command::new(&program).args(args).current_dir(dir).status() {
        Ok(status) => status,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => bail!(
            "`{pm}` was not found on your PATH, so the frontend in {} can't be built.\n\
             Install it, or pass `--skip-frontend` to build only the Rust crate.",
            dir.display()
        ),
        Err(e) => return Err(e).with_context(|| format!("Failed to run `{shown}`")),
    };
    if !status.success() {
        bail!(
            "`{shown}` failed in {}. Fix the errors above, then run `ultimo build` again \
             (or pass `--skip-frontend`).",
            dir.display()
        );
    }
    Ok(())
}

/// The parts of `cargo metadata` the build needs.
struct CargoMetadata {
    target_directory: PathBuf,
    bins: Vec<String>,
}

impl CargoMetadata {
    fn read(dir: &Path) -> Result<Self> {
        let output = Command::new("cargo")
            .args(["metadata", "--format-version", "1", "--no-deps"])
            .current_dir(dir)
            .output()
            .context("Failed to run `cargo metadata`")?;
        if !output.status.success() {
            bail!(
                "`cargo metadata` failed in {}:\n{}",
                dir.display(),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Self::parse(&output.stdout)
    }

    fn parse(json: &[u8]) -> Result<Self> {
        let meta: Value =
            serde_json::from_slice(json).context("`cargo metadata` returned invalid JSON")?;
        let target_directory = meta["target_directory"]
            .as_str()
            .map(PathBuf::from)
            .context("`cargo metadata` output has no target_directory")?;
        let bins = meta["packages"]
            .as_array()
            .into_iter()
            .flatten()
            .flat_map(|pkg| pkg["targets"].as_array().into_iter().flatten())
            .filter(|t| {
                t["kind"]
                    .as_array()
                    .is_some_and(|kinds| kinds.iter().any(|k| k == "bin"))
            })
            .filter_map(|t| t["name"].as_str().map(str::to_string))
            .collect();
        Ok(Self {
            target_directory,
            bins,
        })
    }

    fn has_bin(&self, name: &str) -> bool {
        self.bins.iter().any(|b| b == name)
    }
}

fn step(what: &str, command: &str) {
    println!();
    println!(
        "{} {}",
        format!("▶ {what}").bold(),
        format!("({command})").dimmed()
    );
}

fn skipped(what: &str) {
    println!();
    println!("{}", format!("⏭  Skipped {what}").dimmed());
}

fn absolutize(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        let cwd = std::env::current_dir().context("Failed to read current directory")?;
        // Drop `.` components so `--project .` prints as a clean path.
        Ok(cwd
            .join(path)
            .components()
            .filter(|c| !matches!(c, std::path::Component::CurDir))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn layout_single_crate() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("Cargo.toml"), "").unwrap();
        let layout = detect_layout(tmp.path()).unwrap();
        assert_eq!(layout.backend, tmp.path());
        assert_eq!(layout.frontend, None);
    }

    #[test]
    fn layout_fullstack() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join("backend")).unwrap();
        fs::create_dir_all(tmp.path().join("frontend")).unwrap();
        fs::write(tmp.path().join("backend/Cargo.toml"), "").unwrap();
        fs::write(tmp.path().join("frontend/package.json"), "{}").unwrap();
        let layout = detect_layout(tmp.path()).unwrap();
        assert_eq!(layout.backend, tmp.path().join("backend"));
        assert_eq!(layout.frontend, Some(tmp.path().join("frontend")));
    }

    #[test]
    fn layout_without_cargo_toml_is_an_error() {
        let tmp = tempfile::tempdir().unwrap();
        let err = detect_layout(tmp.path()).unwrap_err().to_string();
        assert!(err.contains("No Cargo.toml"), "{err}");
        assert!(err.contains("--project"), "{err}");
    }

    #[test]
    fn package_manager_follows_the_lockfile() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(package_manager(tmp.path()), "npm");
        for (lock, pm) in [
            ("pnpm-lock.yaml", "pnpm"),
            ("yarn.lock", "yarn"),
            ("bun.lock", "bun"),
        ] {
            let dir = tempfile::tempdir().unwrap();
            fs::write(dir.path().join(lock), "").unwrap();
            assert_eq!(package_manager(dir.path()), pm, "{lock}");
        }
    }

    #[test]
    fn metadata_lists_bin_targets_only() {
        let json = br#"{
            "target_directory": "/work/target",
            "packages": [{
                "targets": [
                    {"name": "app", "kind": ["bin"]},
                    {"name": "generate-client", "kind": ["bin"]},
                    {"name": "app_lib", "kind": ["lib"]}
                ]
            }]
        }"#;
        let meta = CargoMetadata::parse(json).unwrap();
        assert_eq!(meta.target_directory, PathBuf::from("/work/target"));
        assert_eq!(meta.bins, ["app", "generate-client"]);
        assert!(meta.has_bin("generate-client"));
        assert!(!meta.has_bin("app_lib"));
    }

    #[test]
    fn profile_parsing() {
        assert_eq!(Profile::parse("release").unwrap(), Profile::Release);
        assert_eq!(Profile::parse("debug").unwrap(), Profile::Debug);
        assert!(Profile::parse("fast").is_err());
    }
}
