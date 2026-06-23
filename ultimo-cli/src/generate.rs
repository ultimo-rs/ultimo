use anyhow::{Context, Result};
use colored::Colorize;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Run the project's `generate-client` binary to emit a TypeScript client.
///
/// Convention: the project defines `src/bin/generate-client.rs` which builds its
/// `RpcRegistry` and calls `rpc.generate_client_file(<output>)`. This command
/// runs that binary (`cargo run --bin generate-client -- <output>`) so the
/// client is produced by the real registry — no source parsing, no guessing.
pub async fn run(project: PathBuf, output: PathBuf, watch: bool) -> Result<()> {
    println!("📂 Project: {}", project.display().to_string().cyan());
    println!("📝 Output: {}", output.display().to_string().cyan());

    // Run once first
    generate_once(&project, &output)?;

    if !watch {
        return Ok(());
    }

    // Watch mode
    println!();
    println!(
        "{}",
        "👀 Watching for changes (src/**/*.rs)...".bold().cyan()
    );

    use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode};
    use std::time::Duration;

    let (tx, rx) = std::sync::mpsc::channel();
    let mut debouncer = new_debouncer(Duration::from_millis(500), tx)?;

    let src_dir = project.join("src");
    if src_dir.exists() {
        debouncer
            .watcher()
            .watch(&src_dir, RecursiveMode::Recursive)?;
    }

    loop {
        match rx.recv() {
            Ok(Ok(_events)) => {
                println!();
                println!("{}", "♻  Change detected, regenerating...".cyan());
                match generate_once(&project, &output) {
                    Ok(()) => {}
                    Err(e) => {
                        println!("{}", format!("❌ {e}").red());
                        println!("{}", "   Waiting for next change...".dimmed());
                    }
                }
            }
            Ok(Err(e)) => {
                println!("{}", format!("Watch error: {e:?}").red());
            }
            Err(_) => break,
        }
    }

    Ok(())
}

/// Run the generate-client binary once.
fn generate_once(project: &Path, output: &Path) -> Result<()> {
    let cargo_toml = project.join("Cargo.toml");
    if !cargo_toml.exists() {
        anyhow::bail!("No Cargo.toml found in {}", project.display());
    }

    let output_abs = absolutize(output)?;
    if let Some(parent) = output_abs.parent() {
        std::fs::create_dir_all(parent).context("Failed to create output directory")?;
    }

    println!("{}", "🔨 Running generate-client binary...".bold());

    let status = Command::new("cargo")
        .arg("run")
        .arg("--quiet")
        .arg("--bin")
        .arg("generate-client")
        .current_dir(project)
        .arg("--")
        .arg(&output_abs)
        .status()
        .context("Failed to invoke `cargo run --bin generate-client`")?;

    if !status.success() {
        anyhow::bail!(
            "`cargo run --bin generate-client` failed in {project}.\n\n\
             `ultimo generate` runs a `generate-client` binary in your project. \
             Add `src/bin/generate-client.rs`:\n\n\
             {snippet}\n\n\
             It must accept the output path as its first argument.",
            project = project.display(),
            snippet = EXAMPLE_BIN.trim_end(),
        );
    }

    if !output_abs.exists() {
        anyhow::bail!(
            "generate-client ran but did not write {}. Make sure your \
             generate-client binary writes the client to the path given as its \
             first CLI argument (e.g. `rpc.generate_client_file(&args[1])`).",
            output_abs.display()
        );
    }

    println!(
        "{}",
        "✨ TypeScript client generated successfully!"
            .green()
            .bold()
    );
    println!("📄 {}", output_abs.display().to_string().cyan());
    Ok(())
}

/// Example `src/bin/generate-client.rs` shown in the error message.
const EXAMPLE_BIN: &str = r#"    // src/bin/generate-client.rs
    fn main() {
        let out = std::env::args().nth(1).expect("usage: generate-client <output>");
        let rpc = my_app::build_registry(); // build the same RpcRegistry your server uses
        rpc.generate_client_file(&out).expect("failed to write client");
    }"#;

/// Make `path` absolute relative to the current working directory, without
/// requiring it to exist yet (so the output file need not exist).
fn absolutize(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        let cwd = std::env::current_dir().context("Failed to read current directory")?;
        Ok(cwd.join(path))
    }
}
