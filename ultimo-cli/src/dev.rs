use anyhow::Result;
use colored::Colorize;
use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode};
use std::path::Path;
use std::time::Duration;
use tokio::process::Command;
use tokio::signal;
use tokio::sync::mpsc as tokio_mpsc;

/// Run the development server with hot reload.
///
/// Watches `.rs` files and `Cargo.toml` for changes, then recompiles and
/// restarts the server automatically.
pub async fn run(port: u16, host: String) -> Result<()> {
    println!(
        "{}",
        format!("🔥 Starting dev server on {}:{}", host, port)
            .bold()
            .green()
    );
    println!(
        "{}",
        "   Watching for file changes (src/**/*.rs, Cargo.toml)...".dimmed()
    );
    println!();

    let (tx, mut rx) = tokio_mpsc::channel::<()>(1);

    let _debouncer = {
        let tx = tx.clone();
        let (std_tx, std_rx) = std::sync::mpsc::channel();

        let mut debouncer = new_debouncer(Duration::from_millis(500), std_tx)?;

        // Watch src/ directory
        let src_path = Path::new("src");
        if src_path.exists() {
            debouncer
                .watcher()
                .watch(src_path, RecursiveMode::Recursive)?;
        }

        // Watch Cargo.toml
        let cargo_toml = Path::new("Cargo.toml");
        if cargo_toml.exists() {
            debouncer
                .watcher()
                .watch(cargo_toml, RecursiveMode::NonRecursive)?;
        }

        // Bridge std channel to tokio channel in a background thread
        std::thread::spawn(move || {
            while std_rx.recv().is_ok() {
                let _ = tx.blocking_send(());
            }
        });

        debouncer
    };

    // Initial build and run
    let mut child = spawn_server(port, &host).await?;

    loop {
        tokio::select! {
            _ = signal::ctrl_c() => {
                println!("\n{}", "⏹  Shutting down dev server...".yellow());
                kill_child(&mut child).await;
                break;
            }
            _ = rx.recv() => {
                println!();
                println!("{}", "♻  Change detected, restarting...".cyan());
                kill_child(&mut child).await;
                child = spawn_server(port, &host).await?;
            }
        }
    }

    Ok(())
}

async fn spawn_server(port: u16, host: &str) -> Result<tokio::process::Child> {
    println!("{}", "   Compiling...".dimmed());

    // Build first so we can show compilation errors clearly
    let build_status = Command::new("cargo").args(["build"]).status().await?;

    if !build_status.success() {
        println!(
            "{}",
            "❌ Build failed. Waiting for file changes...".red().bold()
        );
        // Return a dummy process that just sleeps (will be killed on next change)
        let child = Command::new("sleep").arg("86400").spawn()?;
        return Ok(child);
    }

    println!(
        "{}",
        format!("✅ Running on http://{}:{}", host, port).green()
    );
    println!();

    let child = Command::new("cargo")
        .args(["run"])
        .env("PORT", port.to_string())
        .env("HOST", host)
        .spawn()?;

    Ok(child)
}

async fn kill_child(child: &mut tokio::process::Child) {
    let _ = child.kill().await;
    let _ = child.wait().await;
}
