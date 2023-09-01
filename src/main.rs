mod commands;
mod libs;
mod templates;
mod tools;
use clap::{Parser, Subcommand};
use commands::{create, start};
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
    #[command(arg_required_else_help = true)]
    Create(create::CreateArgs),
    Start(start::StartArgs),
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Create(args) => create::cmd(args),
        Commands::Start(args) => start::cmd(args),
    }
}
