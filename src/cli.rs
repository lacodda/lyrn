use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::model::{Addon, Form};

/// Start a new web application on the lacodda line's stack: Vite, React,
/// TypeScript, Tailwind and the dowel design system, with the line's standard
/// already in place.
#[derive(Parser)]
#[command(name = "lyrn", version, about, propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

/// Boxed: the argument structs grew past the point where carrying them inline
/// makes every `Command` the size of the largest variant, and the enum is
/// moved around far more often than a project is created.
#[derive(Subcommand)]
pub enum Command {
    /// Create a new project in a directory of its own
    New(Box<NewArgs>),
    /// Start a project in a directory that already exists
    Init(Box<InitArgs>),
    /// Bring an existing repository up to the line's standard
    Adopt(Box<AdoptArgs>),
    /// List the forms a project can take
    Forms,
}

#[derive(clap::Args)]
pub struct NewArgs {
    /// The project name: lowercase letters, digits and hyphens
    pub name: String,

    /// Where to create it; defaults to a directory named after the project,
    /// and for `--form docs` to the repository in the current directory
    #[arg(long, value_name = "PATH")]
    pub path: Option<PathBuf>,

    #[command(flatten)]
    pub project: ProjectArgs,
}

#[derive(clap::Args)]
pub struct InitArgs {
    /// The directory to start the project in; defaults to the current one
    pub path: Option<PathBuf>,

    /// The project name; defaults to the directory's
    #[arg(long)]
    pub name: Option<String>,

    #[command(flatten)]
    pub project: ProjectArgs,
}

#[derive(clap::Args)]
pub struct AdoptArgs {
    /// The repository to adopt; defaults to the current directory
    pub path: Option<PathBuf>,

    /// The form the repository has; read from the repository when not given
    #[arg(long, value_name = "FORM")]
    pub form: Option<Form>,

    /// The application a plugin extends; read from a plugin's name when not given
    #[arg(long, value_name = "HOST")]
    pub host: Option<String>,

    /// The project name; read from its manifest, else the directory's
    #[arg(long)]
    pub name: Option<String>,

    #[command(flatten)]
    pub identity: IdentityArgs,

    /// Accept the defaults instead of asking
    #[arg(short = 'y', long = "yes")]
    pub assume_yes: bool,

    /// Show what would be added, and write nothing
    #[arg(long)]
    pub dry_run: bool,
}

/// What a generated project is: its form and the pieces it comes with, who it
/// belongs to, and how the run behaves. Shared by `new` and `init`, which
/// differ only in where the files go.
#[derive(clap::Args)]
pub struct ProjectArgs {
    /// The shape of the project
    #[arg(long, value_name = "FORM", default_value = "spa")]
    pub form: Form,

    /// The application a plugin extends (see `lyrn forms`); `--form plugin` only
    #[arg(long, value_name = "HOST")]
    pub host: Option<String>,

    #[command(flatten)]
    pub identity: IdentityArgs,

    /// Optional pieces to generate with, comma-separated (see `lyrn forms`)
    #[arg(long = "with", value_name = "ADDON", value_delimiter = ',')]
    pub with: Vec<Addon>,

    /// Accept the defaults instead of asking
    #[arg(short = 'y', long = "yes")]
    pub assume_yes: bool,

    /// Show the tree that would be written, and write nothing
    #[arg(long)]
    pub dry_run: bool,

    /// Skip the hooks that install dependencies and start the repository
    #[arg(long)]
    pub no_hooks: bool,
}

/// Who a project is: the things every file of the standard is written with,
/// whether the project is new or adopted.
#[derive(clap::Args, Default)]
pub struct IdentityArgs {
    /// The product's colour: a product of the line, or a `#rrggbb` value
    #[arg(long, value_name = "COLOUR")]
    pub accent: Option<String>,

    /// One line describing what the project is
    #[arg(long)]
    pub description: Option<String>,

    /// The author recorded in LICENSE; defaults to `git config user.name`
    #[arg(long)]
    pub author: Option<String>,

    /// The GitHub repository it will live in, as `owner/name`
    #[arg(long, value_name = "OWNER/NAME")]
    pub repo: Option<String>,
}
