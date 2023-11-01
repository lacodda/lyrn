use clap::{Args, Subcommand};
use std::error::Error;

#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct ExportArgs {
    #[command(subcommand)]
    command: Option<ExportCommands>,
}

#[derive(Debug, Subcommand)]
enum ExportCommands {
    Config(ExportConfigArgs),
}

#[derive(Debug, Args)]
pub struct ExportConfigArgs {
    #[arg(short, long)]
    show: bool,
}

pub fn cmd(export_args: ExportArgs) -> Result<(), Box<dyn Error>> {
    let export_cmd = export_args.command.unwrap();
    match export_cmd {
        ExportCommands::Config(args) => export_config(args),
    }
}

fn export_config(args: ExportConfigArgs) -> Result<(), Box<dyn Error>> {
    println!("{:?}", args);
    Ok(())
}
