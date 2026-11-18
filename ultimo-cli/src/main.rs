use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;

mod build;
mod dev;
mod generate;
mod mcp;
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

    /// Development server with hot reload
    Dev {
        /// Port to run on
        #[arg(short, long, default_value = "3000")]
        port: u16,

        /// Host to bind to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
    },

    /// Build the project for production: the Rust crate, the TypeScript client and the frontend
    Build {
        /// Build profile (debug or release)
        #[arg(short, long, default_value = "release", value_parser = ["debug", "release"])]
        profile: String,

        /// Project root (a crate, or a fullstack project with backend/ and frontend/)
        #[arg(long, default_value = ".")]
        project: PathBuf,

        /// Also write the TypeScript client here (default: frontend/src/generated/client.ts when there is a frontend)
        #[arg(long)]
        client: Option<PathBuf>,

        /// Don't regenerate the TypeScript client
        #[arg(long)]
        skip_client: bool,

        /// Don't build the frontend
        #[arg(long)]
        skip_frontend: bool,
    },

    /// Run a Model Context Protocol server (stdio) for AI coding agents
    Mcp,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // The MCP server speaks JSON-RPC over stdout — it must emit no banner or any
    // other stdout noise, so handle it before printing anything.
    if let Commands::Mcp = cli.command {
        return mcp::run().await;
    }

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
        Commands::Dev { port, host } => {
            dev::run(port, host).await?;
        }
        Commands::Build {
            profile,
            project,
            client,
            skip_client,
            skip_frontend,
        } => {
            build::run(build::Options {
                project,
                profile: build::Profile::parse(&profile)?,
                client,
                skip_client,
                skip_frontend,
            })
            .await?;
        }
        // Handled before the banner above.
        Commands::Mcp => unreachable!(),
    }

    Ok(())
}
