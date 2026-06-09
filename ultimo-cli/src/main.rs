use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;

mod generate;
mod new;

#[derive(Parser)]
#[command(name = "ultimo")]
#[command(version)]
#[command(about = "Ultimo Framework CLI - Build modern Rust web applications", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate TypeScript client from RPC definitions
    Generate {
        /// Path to the Rust project directory
        #[arg(short, long, default_value = ".")]
        project: PathBuf,

        /// Output directory for generated TypeScript files
        #[arg(short, long)]
        output: PathBuf,

        /// Watch for changes and regenerate automatically
        #[arg(short, long)]
        watch: bool,
    },

    /// Create a new Ultimo project
    New {
        /// Project name
        name: String,

        /// Template to use (basic, fullstack, api-only, rpc, production)
        #[arg(short, long, default_value = "basic")]
        template: String,
    },

    /// [not implemented yet] Development server with hot reload (planned for 0.7.0)
    Dev {
        /// Port to run on
        #[arg(short, long, default_value = "3000")]
        port: u16,
    },

    /// [not implemented yet] Production build — use `cargo build --release` for now
    Build {
        /// Build profile (debug or release)
        #[arg(short, long, default_value = "release")]
        profile: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    println!("{}", "⚡ Ultimo Framework CLI".bold().cyan());
    println!();

    match cli.command {
        Commands::Generate {
            project,
            output,
            watch,
        } => {
            generate::run(project, output, watch).await?;
        }
        Commands::New { name, template } => {
            new::run(name, template).await?;
        }
        Commands::Dev { port: _ } => {
            println!(
                "{}",
                "`ultimo dev` is not implemented yet (planned for 0.7.0).".yellow()
            );
            println!("Run your app directly for now:  {}", "cargo run".cyan());
        }
        Commands::Build { profile: _ } => {
            println!("{}", "`ultimo build` is not implemented yet.".yellow());
            println!(
                "Build with cargo for now:  {}",
                "cargo build --release".cyan()
            );
        }
    }

    Ok(())
}
