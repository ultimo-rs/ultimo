use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;

mod generate;

#[derive(Parser)]
#[command(name = "ultimo")]
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

        /// Template to use (basic, fullstack, api-only)
        #[arg(short, long, default_value = "basic")]
        template: String,
    },

    /// Start development server with hot reload
    Dev {
        /// Port to run on
        #[arg(short, long, default_value = "3000")]
        port: u16,
    },

    /// Build for production
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
            println!("🚀 Creating new project: {}", name.green());
            println!("📦 Template: {}", template);
            println!();
            println!("{}", "Coming soon!".yellow());
        }
        Commands::Dev { port } => {
            println!(
                "🔥 Starting development server on port {}",
                port.to_string().green()
            );
            println!();
            println!("{}", "Coming soon!".yellow());
        }
        Commands::Build { profile } => {
            println!("🔨 Building with profile: {}", profile.green());
            println!();
            println!("{}", "Coming soon!".yellow());
        }
    }

    Ok(())
}
