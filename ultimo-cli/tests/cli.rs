//! CLI integration tests — guard the command surface (flags, help, scaffolding,
//! and the `generate` convention) against drift.
//! Run with: cargo test -p ultimo-cli

use assert_cmd::Command;
use predicates::str::contains;
use std::fs;

fn ultimo() -> Command {
    Command::cargo_bin("ultimo").expect("ultimo binary builds")
}

#[test]
fn help_lists_the_real_subcommands() {
    ultimo()
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("generate"))
        .stdout(contains("new"))
        .stdout(contains("dev"))
        .stdout(contains("build"))
        .stdout(contains("mcp"));
}

#[test]
fn version_flag_prints_version() {
    ultimo()
        .arg("--version")
        .assert()
        .success()
        .stdout(contains("ultimo "));
}

#[test]
fn dev_shows_help() {
    ultimo()
        .args(["dev", "--help"])
        .assert()
        .success()
        .stdout(contains("hot reload"));
}

/// Write a minimal cargo package at `dir`: a server binary plus, optionally, a
/// `generate-client` binary that writes a marker client to its first argument.
fn write_crate(dir: &std::path::Path, main_rs: &str, with_generator: bool) {
    fs::create_dir_all(dir.join("src/bin")).unwrap();
    let mut cargo = String::from(
        "[package]\nname = \"app\"\nversion = \"0.0.0\"\nedition = \"2021\"\ndefault-run = \"app\"\n\n[workspace]\n",
    );
    if with_generator {
        cargo.push_str(
            "\n[[bin]]\nname = \"generate-client\"\npath = \"src/bin/generate-client.rs\"\n",
        );
        fs::write(
            dir.join("src/bin/generate-client.rs"),
            "fn main() {\n    let out = std::env::args().nth(1).expect(\"output path\");\n    std::fs::create_dir_all(std::path::Path::new(&out).parent().unwrap()).unwrap();\n    std::fs::write(&out, \"// generated client\\n\").unwrap();\n}\n",
        )
        .unwrap();
    }
    fs::write(dir.join("Cargo.toml"), cargo).unwrap();
    fs::write(dir.join("src/main.rs"), main_rs).unwrap();
}

fn node_available() -> bool {
    std::process::Command::new("node")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

#[test]
fn build_errors_helpfully_outside_a_project() {
    let tmp = tempfile::tempdir().unwrap();
    ultimo()
        .current_dir(tmp.path())
        .arg("build")
        .assert()
        .failure()
        .stderr(contains("No Cargo.toml"))
        .stderr(contains("--project"));
}

#[test]
fn build_rejects_an_unknown_profile() {
    ultimo()
        .args(["build", "--profile", "fast"])
        .assert()
        .failure()
        .stderr(contains("debug"))
        .stderr(contains("release"));
}

#[test]
fn build_compiles_a_crate_and_reports_the_binary() {
    let tmp = tempfile::tempdir().unwrap();
    write_crate(tmp.path(), "fn main() {}\n", false);

    ultimo()
        .current_dir(tmp.path())
        .env_remove("CARGO_TARGET_DIR")
        .args(["build", "--profile", "debug"])
        .assert()
        .success()
        .stdout(contains("Build complete"))
        .stdout(contains(format!("app{}", std::env::consts::EXE_SUFFIX)));

    let bin = tmp
        .path()
        .join("target/debug")
        .join(format!("app{}", std::env::consts::EXE_SUFFIX));
    assert!(bin.is_file(), "expected {} to exist", bin.display());
}

#[test]
fn build_stops_with_a_clear_error_when_rust_fails() {
    let tmp = tempfile::tempdir().unwrap();
    write_crate(tmp.path(), "fn main() { this is not rust }\n", false);

    ultimo()
        .current_dir(tmp.path())
        .env_remove("CARGO_TARGET_DIR")
        .args(["build", "--profile", "debug"])
        .assert()
        .failure()
        .stderr(contains("`cargo build` failed"));
}

#[test]
fn build_client_flag_regenerates_the_client_for_a_single_crate() {
    let tmp = tempfile::tempdir().unwrap();
    write_crate(tmp.path(), "fn main() {}\n", true);

    ultimo()
        .current_dir(tmp.path())
        .env_remove("CARGO_TARGET_DIR")
        .args(["build", "--profile", "debug", "--client", "web/client.ts"])
        .assert()
        .success();

    let client = fs::read_to_string(tmp.path().join("web/client.ts")).unwrap();
    assert!(client.contains("generated client"));
}

#[test]
fn build_client_flag_without_a_generator_is_an_error() {
    let tmp = tempfile::tempdir().unwrap();
    write_crate(tmp.path(), "fn main() {}\n", false);

    ultimo()
        .current_dir(tmp.path())
        .env_remove("CARGO_TARGET_DIR")
        .args(["build", "--profile", "debug", "--client", "client.ts"])
        .assert()
        .failure()
        .stderr(contains("generate-client"));
}

/// The fullstack order matters: the client must be regenerated *before* the
/// frontend builds, or the frontend compiles against a stale client. The
/// frontend's build script copies the client into `dist/` to prove it existed.
#[test]
fn build_fullstack_generates_the_client_then_builds_the_frontend() {
    if !node_available() {
        eprintln!("skipping: node is not installed");
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    write_crate(&tmp.path().join("backend"), "fn main() {}\n", true);
    fs::create_dir_all(tmp.path().join("frontend")).unwrap();
    fs::write(
        tmp.path().join("frontend/package.json"),
        r#"{
  "name": "frontend",
  "private": true,
  "scripts": {
    "build": "node -e \"const fs=require('fs');fs.mkdirSync('dist',{recursive:true});fs.copyFileSync('src/generated/client.ts','dist/client.ts')\""
  }
}
"#,
    )
    .unwrap();

    ultimo()
        .current_dir(tmp.path())
        .env_remove("CARGO_TARGET_DIR")
        .args(["build", "--profile", "debug"])
        .assert()
        .success()
        .stdout(contains("dist"));

    let built = fs::read_to_string(tmp.path().join("frontend/dist/client.ts")).unwrap();
    assert!(built.contains("generated client"));
}

#[test]
fn build_skip_frontend_builds_only_rust() {
    let tmp = tempfile::tempdir().unwrap();
    write_crate(&tmp.path().join("backend"), "fn main() {}\n", true);
    fs::create_dir_all(tmp.path().join("frontend")).unwrap();
    // A build script that would fail if it ran.
    fs::write(
        tmp.path().join("frontend/package.json"),
        r#"{"scripts": {"build": "exit 1"}}"#,
    )
    .unwrap();

    ultimo()
        .current_dir(tmp.path())
        .env_remove("CARGO_TARGET_DIR")
        .args(["build", "--profile", "debug", "--skip-frontend"])
        .assert()
        .success()
        .stdout(contains("Skipped frontend"));

    // The client is still regenerated for the frontend to pick up later.
    assert!(tmp
        .path()
        .join("frontend/src/generated/client.ts")
        .is_file());
}

#[test]
fn new_scaffolds_a_project() {
    let tmp = tempfile::tempdir().unwrap();
    ultimo()
        .current_dir(tmp.path())
        .args(["new", "demo-app", "--template", "basic"])
        .assert()
        .success();

    assert!(tmp.path().join("demo-app/Cargo.toml").exists());
    assert!(tmp.path().join("demo-app/src/main.rs").exists());
}

#[test]
fn new_scaffold_pins_current_ultimo_version() {
    let tmp = tempfile::tempdir().unwrap();
    ultimo()
        .current_dir(tmp.path())
        .args(["new", "demo", "--template", "basic"])
        .assert()
        .success();

    let cargo = fs::read_to_string(tmp.path().join("demo/Cargo.toml")).unwrap();
    // The scaffold must pin the current `ultimo` major.minor, derived from this
    // CLI's own version so it can never drift back to a stale pin.
    let v = env!("CARGO_PKG_VERSION");
    let mut it = v.split('.');
    let majmin = format!("{}.{}", it.next().unwrap(), it.next().unwrap());
    assert!(
        cargo.contains(&format!("ultimo = \"{majmin}\"")),
        "expected `ultimo = \"{majmin}\"` in Cargo.toml:\n{cargo}"
    );
    assert!(
        !cargo.contains("ultimo = \"0.1\""),
        "stale `ultimo = \"0.1\"` pin still present:\n{cargo}"
    );
}

#[test]
fn new_scaffold_writes_agents_md() {
    let tmp = tempfile::tempdir().unwrap();
    ultimo()
        .current_dir(tmp.path())
        .args(["new", "demo", "--template", "basic"])
        .assert()
        .success();

    let agents = fs::read_to_string(tmp.path().join("demo/AGENTS.md"))
        .expect("scaffold must write an AGENTS.md for coding agents");
    assert!(
        agents.contains("Ultimo"),
        "AGENTS.md should orient an agent: {agents}"
    );
}

#[test]
fn rpc_scaffold_has_a_real_generate_client_bin() {
    let tmp = tempfile::tempdir().unwrap();
    ultimo()
        .current_dir(tmp.path())
        .args(["new", "demo", "--template", "rpc"])
        .assert()
        .success();

    let gen = fs::read_to_string(tmp.path().join("demo/src/bin/generate-client.rs")).unwrap();
    assert!(
        gen.contains("generate_client_file"),
        "generate-client bin must call the real generator, not a placeholder:\n{gen}"
    );
    assert!(
        !gen.contains("Would generate"),
        "placeholder generate-client bin still present:\n{gen}"
    );
}

#[test]
fn fullstack_scaffold_actually_uses_rpc_registry() {
    let tmp = tempfile::tempdir().unwrap();
    ultimo()
        .current_dir(tmp.path())
        .args(["new", "demo", "--template", "fullstack"])
        .assert()
        .success();

    let backend_cargo = fs::read_to_string(tmp.path().join("demo/backend/Cargo.toml")).unwrap();
    assert!(
        backend_cargo.contains("client-gen"),
        "fullstack backend must enable client-gen to use RpcRegistry::query/mutation:\n{backend_cargo}"
    );

    let api = fs::read_to_string(tmp.path().join("demo/backend/src/api.rs")).unwrap();
    assert!(
        api.contains("RpcRegistry") && api.contains(".query(") && api.contains(".mutation("),
        "fullstack backend must build a real RpcRegistry with at least one query and mutation:\n{api}"
    );

    let main = fs::read_to_string(tmp.path().join("demo/backend/src/main.rs")).unwrap();
    assert!(
        main.contains("mount_rpc") && main.contains("generate_client_file"),
        "fullstack backend must mount the registry via mount_rpc and generate a real client:\n{main}"
    );

    let gen =
        fs::read_to_string(tmp.path().join("demo/backend/src/bin/generate-client.rs")).unwrap();
    assert!(
        gen.contains("generate_client_file"),
        "generate-client bin must call the real generator, not a placeholder:\n{gen}"
    );

    let app = fs::read_to_string(tmp.path().join("demo/frontend/src/App.tsx")).unwrap();
    assert!(
        app.contains("UltimoRpcClient"),
        "frontend must call the backend through the generated typed client:\n{app}"
    );
}

/// Every template that registers `logger()` must also install a `tracing`
/// subscriber (in both `Cargo.toml` and `main.rs`) — otherwise `logger()` is a
/// silent no-op and a user following the docs exactly gets zero output.
#[test]
fn every_scaffold_using_logger_installs_a_tracing_subscriber() {
    let tmp = tempfile::tempdir().unwrap();
    let templates_and_main_rs = [
        ("basic", "demo/src/main.rs", "demo/Cargo.toml"),
        ("api-only", "demo/src/main.rs", "demo/Cargo.toml"),
        ("rpc", "demo/src/main.rs", "demo/Cargo.toml"),
        ("production", "demo/src/main.rs", "demo/Cargo.toml"),
        (
            "fullstack",
            "demo/backend/src/main.rs",
            "demo/backend/Cargo.toml",
        ),
    ];

    for (template, main_rs_path, cargo_toml_path) in templates_and_main_rs {
        let dir = tmp.path().join(template);
        fs::create_dir_all(&dir).unwrap();
        ultimo()
            .current_dir(&dir)
            .args(["new", "demo", "--template", template])
            .assert()
            .success();

        let main_rs = fs::read_to_string(dir.join(main_rs_path)).unwrap();
        if !main_rs.contains("builtin::logger()") {
            continue;
        }

        assert!(
            main_rs.contains("tracing_subscriber::fmt()"),
            "{template} registers logger() but never installs a tracing subscriber in main.rs:\n{main_rs}"
        );

        let cargo_toml = fs::read_to_string(dir.join(cargo_toml_path)).unwrap();
        assert!(
            cargo_toml.contains("tracing-subscriber"),
            "{template} registers logger() but Cargo.toml doesn't depend on tracing-subscriber:\n{cargo_toml}"
        );
    }
}

#[test]
fn production_scaffold_registers_request_id_before_logger() {
    let tmp = tempfile::tempdir().unwrap();
    ultimo()
        .current_dir(tmp.path())
        .args(["new", "demo", "--template", "production"])
        .assert()
        .success();

    let main = fs::read_to_string(tmp.path().join("demo/src/main.rs")).unwrap();
    let rid = main
        .find("builtin::request_id()")
        .expect("production scaffold should register request_id()");
    let log = main.find("builtin::logger()").expect("logger()");
    assert!(
        rid < log,
        "request_id() must be registered before logger() so its span wraps it"
    );
}

#[test]
fn generate_runs_the_convention_bin_and_writes_output() {
    // A minimal cargo project whose generate-client bin writes its first arg.
    let tmp = tempfile::tempdir().unwrap();
    let proj = tmp.path().join("proj");
    fs::create_dir_all(proj.join("src/bin")).unwrap();
    fs::write(
        proj.join("Cargo.toml"),
        "[package]\nname = \"proj\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[[bin]]\nname = \"generate-client\"\npath = \"src/bin/generate-client.rs\"\n",
    )
    .unwrap();
    fs::write(
        proj.join("src/bin/generate-client.rs"),
        "fn main() {\n    let out = std::env::args().nth(1).expect(\"output path\");\n    std::fs::write(&out, \"// generated client\\n\").unwrap();\n}\n",
    )
    .unwrap();

    let out = tmp.path().join("client.ts");
    ultimo()
        .args(["generate", "--project"])
        .arg(&proj)
        .arg("--output")
        .arg(&out)
        .assert()
        .success();

    let written = fs::read_to_string(&out).expect("client written");
    assert!(written.contains("generated client"));
}

#[test]
fn generate_errors_helpfully_without_the_bin() {
    // A cargo project with no generate-client bin.
    let tmp = tempfile::tempdir().unwrap();
    let proj = tmp.path().join("proj");
    fs::create_dir_all(proj.join("src")).unwrap();
    fs::write(
        proj.join("Cargo.toml"),
        "[package]\nname = \"proj\"\nversion = \"0.0.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::write(proj.join("src/main.rs"), "fn main() {}\n").unwrap();

    let out = tmp.path().join("client.ts");
    ultimo()
        .args(["generate", "--project"])
        .arg(&proj)
        .arg("--output")
        .arg(&out)
        .assert()
        .failure()
        .stderr(contains("generate-client"));
}
