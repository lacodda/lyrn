use crate::tools::webpack;
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
    let export_cmd: ExportCommands = export_args.command.unwrap();
    match export_cmd {
        ExportCommands::Config(args) => export_config(args),
    }
}

fn export_config(_args: ExportConfigArgs) -> Result<(), Box<dyn Error>> {
    let _ = webpack::export_config(webpack::Env::Dev);
    let _ = webpack::export_config(webpack::Env::Prod);

    Ok(())
}
