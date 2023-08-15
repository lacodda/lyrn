use clap::{Parser, Subcommand};
use std::error::Error;

mod commands;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
#[command(arg_required_else_help(true))]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Create(commands::create::Create),
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Create(project) => commands::create::project(project),
    }
}
