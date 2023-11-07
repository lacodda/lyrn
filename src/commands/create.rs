use crate::libs::project::{create_project, CreateProjectArgs};
use clap::{Args, Subcommand};
use std::error::Error;

#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct CreateArgs {
    #[command(subcommand)]
    command: Option<CreateCommands>,
}

#[derive(Debug, Subcommand)]
enum CreateCommands {
    #[command(about = "Create a new project", arg_required_else_help = true)]
    Project(CreateProjectArgs),
}

pub fn cmd(create_args: CreateArgs) -> Result<(), Box<dyn Error>> {
    let create_cmd = create_args.command.unwrap();
    match create_cmd {
        CreateCommands::Project(args) => create_project(args),
    }
}
