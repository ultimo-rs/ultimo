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
        .stdout(contains("build"));
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

#[test]
fn build_reports_not_implemented() {
    ultimo()
        .arg("build")
        .assert()
        .success()
        .stdout(contains("not implemented"));
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
