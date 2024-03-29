use crate::{tools::webpack, libs::project_config::EnvType};
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
    #[command(about = "Export configuration files")]
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

fn export_config(args: ExportConfigArgs) -> Result<(), Box<dyn Error>> {
    if args.show {
        let _ = webpack::show_config(EnvType::Dev);
        let _ = webpack::show_config(EnvType::Prod);
        return Ok(());
    }
    let _ = webpack::export_config(EnvType::Dev);
    let _ = webpack::export_config(EnvType::Prod);
    Ok(())
}
