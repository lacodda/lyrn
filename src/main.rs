mod commands;
mod libs;
mod templates;
mod tools;
mod traits;
use clap::{Parser, Subcommand};
use commands::{create, start, build, export};
use std::error::Error;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
#[command(arg_required_else_help(true))]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[command(about = "Create a new project", arg_required_else_help = true)]
    Create(create::CreateArgs),
    #[command(about = "Run the project in development mode")]
    Start(start::StartArgs),
    #[command(about = "Build project")]
    Build(build::BuildArgs),
    #[command(about = "Export configuration files", arg_required_else_help = true)]
    Export(export::ExportArgs),
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Create(args) => create::cmd(args),
        Commands::Start(args) => start::cmd(args),
        Commands::Build(args) => build::cmd(args),
        Commands::Export(args) => export::cmd(args),
    }
}
