use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub async fn run(project: PathBuf, output: PathBuf, watch: bool) -> Result<()> {
    println!("📂 Project: {}", project.display().to_string().cyan());
    println!("📝 Output: {}", output.display().to_string().cyan());

    if watch {
        println!("👀 Watch mode enabled");
        println!();
        println!(
            "{}",
            "Coming soon! Use the generate command without --watch for now.".yellow()
        );
        return Ok(());
    }

    println!();
    println!("{}", "🔍 Analyzing Rust project...".bold());

    // Step 1: Check if Cargo.toml exists
    let cargo_toml = project.join("Cargo.toml");
    if !cargo_toml.exists() {
        anyhow::bail!("No Cargo.toml found in {}", project.display());
    }

    println!("✅ Found Cargo.toml");

    // Step 2: Build the project to extract RPC metadata
    println!("{}", "🔨 Building project...".bold());

    let output_result = Command::new("cargo")
        .arg("build")
        .arg("--release")
        .current_dir(&project)
        .output()
        .context("Failed to build project")?;

    if !output_result.status.success() {
        let error = String::from_utf8_lossy(&output_result.stderr);
        anyhow::bail!("Build failed:\n{}", error);
    }

    println!("✅ Build successful");

    // Step 3: Look for TypeScript client code in build artifacts or generated files
    println!("{}", "🔍 Searching for RPC definitions...".bold());

    // Check common locations for generated client code
    let possible_locations = vec![
        project.join("target/ultimo-client.ts"),
        project.join("ultimo-client.ts"),
    ];

    let mut client_code = None;
    for location in possible_locations {
        if location.exists() {
            client_code = Some(fs::read_to_string(&location)?);
            println!("✅ Found TypeScript client at {}", location.display());
            break;
        }
    }

    // If no pre-generated client found, try to extract from running the binary
    if client_code.is_none() {
        println!("⚠️  No pre-generated client found");
        println!("💡 Tip: Make sure your main.rs calls rpc.generate_typescript_client()");
        println!();
        println!("Add this to your main.rs:");
        println!(
            "{}",
            "    let client = rpc.generate_typescript_client();".bright_black()
        );
        println!(
            "{}",
            "    fs::write(\"ultimo-client.ts\", client)?;".bright_black()
        );
    }

    // Step 4: Create output directory if needed
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).context("Failed to create output directory")?;
    }

    // Step 5: Write the TypeScript client
    if let Some(code) = client_code {
        fs::write(&output, code).context("Failed to write TypeScript client")?;

        println!();
        println!(
            "{}",
            "✨ TypeScript client generated successfully!"
                .green()
                .bold()
        );
        println!("📄 {}", output.display().to_string().cyan());
    }

    Ok(())
}

/// Generate TypeScript client from RPC registry at runtime
#[allow(dead_code)]
pub fn generate_from_code(rpc_definitions: &str) -> Result<String> {
    // Parse RPC definitions and generate TypeScript code
    // This would be called during the build process

    Ok(rpc_definitions.to_string())
}
