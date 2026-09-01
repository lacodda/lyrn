use clap::{Parser, Subcommand};

use crate::model::Form;

/// Start a new web application on the lacodda line's stack: Vite, React,
/// TypeScript, Tailwind and the dowel design system, with the line's standard
/// already in place.
#[derive(Parser)]
#[command(name = "lyrn", version, about, propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Create a new project
    New(NewArgs),
    /// List the forms a project can take
    Forms,
}

#[derive(clap::Args)]
pub struct NewArgs {
    /// The project name: lowercase letters, digits and hyphens
    pub name: String,

    /// The shape of the project
    #[arg(long, value_name = "FORM", default_value = "spa")]
    pub form: Form,

    /// The product's colour: a product of the line, or a `#rrggbb` value
    #[arg(long, value_name = "COLOUR")]
    pub accent: Option<String>,

    /// One line describing what the project is
    #[arg(long)]
    pub description: Option<String>,

    /// The author recorded in LICENSE; defaults to `git config user.name`
    #[arg(long)]
    pub author: Option<String>,

    /// Where to create it; defaults to a directory named after the project
    #[arg(long, value_name = "PATH")]
    pub path: Option<std::path::PathBuf>,

    /// Accept the defaults instead of asking
    #[arg(short = 'y', long = "yes")]
    pub assume_yes: bool,

    /// Show what would be written, and write nothing
    #[arg(long)]
    pub dry_run: bool,

    /// Skip the hooks that install dependencies and start the repository
    #[arg(long)]
    pub no_hooks: bool,
}
