use std::path::PathBuf;

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
    /// Catch a repository up with the newest version of the line's standard,
    /// or of the template it was made from
    Upgrade(UpgradeArgs),
    /// Hold a repository to the line's standard, and say what is off and why
    Doctor(DoctorArgs),
    /// Hold the shopfronts to each other: versions, descriptions, commands, the site's address
    Check(CheckArgs),
    /// Put the repository on GitHub, or bring the one there in line: what it says, its pages, its rules
    Repo(RepoArgs),
    /// Open one of the repository's shopfronts in the browser
    Open(OpenArgs),
    /// Remove what a build or an install writes again, and say what it weighed
    Clean(CleanArgs),
    /// Lay the mark out by the line's canon and draw every raster of it from its masters
    Assets(AssetsArgs),
    /// A new product's mark, held to the line's: its code, its colour, and the two together
    #[command(subcommand)]
    Brand(BrandCommand),
    /// Run here the CI a push of this commit would start, before pushing it
    Ci(CiArgs),
    /// Start a stage: what it is, what stands in its way, and a line in rigger's record
    Stage(PathArgs),
    /// Where the repository stands: the branch, the release, its CI, the stack, the stage
    Status(PathArgs),
    /// Where the version is written, and where it went: tags and registries
    Versions(PathArgs),
    /// When the stack was last brought up to date, and which dependencies are behind
    Deps(PathArgs),
    /// Measure the oldest Rust the code builds on, and write it as rust-version
    Msrv(PathArgs),
    /// Write a decision down: the next number in the log, listed in its index
    #[command(subcommand)]
    Adr(AdrCommand),
    /// The documentation site: a page written, the site held to the program, the site served
    #[command(subcommand)]
    Docs(DocsCommand),
    /// Take the README's transcripts again from the build
    #[command(subcommand)]
    Readme(ReadmeCommand),
    /// A version's changelog section, and the screens that go with it
    Changelog(ChangelogArgs),
    /// List the forms a project can take, and your own templates
    Forms,
    /// Check a template, or write a built-in form out as one
    #[command(subcommand)]
    Template(TemplateCommand),
}

#[derive(Subcommand)]
pub enum AdrCommand {
    /// Write a new decision under the next free number, and list it in the index
    New {
        /// What was decided, as the record's title
        title: String,

        /// The repository; defaults to the one the current directory is in
        #[arg(long, value_name = "PATH")]
        path: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
pub enum DocsCommand {
    /// Write a command's reference page from its --help; with no command, every page missing
    Ref {
        /// The command; defaults to every command with no page yet
        command: Option<String>,

        /// The repository; defaults to the one the current directory is in
        #[arg(long, value_name = "PATH")]
        path: Option<PathBuf>,
    },
    /// Start a guide: a page that walks through doing something
    Guide {
        /// The page's title
        title: String,

        /// One line for search results and the page's metadata
        #[arg(long)]
        description: Option<String>,

        /// The repository; defaults to the one the current directory is in
        #[arg(long, value_name = "PATH")]
        path: Option<PathBuf>,
    },
    /// Start a concept: a page that says what something is and why
    Concept {
        /// The page's title
        title: String,

        /// One line for search results and the page's metadata
        #[arg(long)]
        description: Option<String>,

        /// The repository; defaults to the one the current directory is in
        #[arg(long, value_name = "PATH")]
        path: Option<PathBuf>,
    },
    /// Hold the site to the program: commands, subcommands and flags with no page, pages and links to nothing
    Check {
        /// The repository; defaults to the one the current directory is in
        path: Option<PathBuf>,
    },
    /// Serve the site as you write it, installing it first if it has not been
    Dev {
        /// The repository; defaults to the one the current directory is in
        path: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
pub enum ReadmeCommand {
    /// Run the console blocks marked `<!-- snap -->` and write what they print back
    Snap {
        /// The repository; defaults to the one the current directory is in
        path: Option<PathBuf>,

        /// Say what no longer matches, write nothing, and exit with 1 if anything does
        #[arg(long)]
        check: bool,
    },
}

#[derive(clap::Args)]
pub struct ChangelogArgs {
    /// The version; defaults to the newest in CHANGELOG.md
    #[arg(value_name = "VERSION")]
    pub release: Option<String>,

    /// Put the version's images from docs/screens/<tag>/ into its section, instead of printing it
    #[arg(long)]
    pub screens: bool,

    /// The repository; defaults to the one the current directory is in
    #[arg(long, value_name = "PATH")]
    pub path: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum TemplateCommand {
    /// Check that a template can be used: its manifest, its variables, its
    /// sections, its hooks
    Check {
        /// The template's directory; defaults to the current one
        path: Option<PathBuf>,
    },
    /// Write a built-in form out as a template repository of its own, or over
    /// an earlier export of it
    Export {
        /// The form to write out
        form: Form,

        /// Where to write it: a new directory, or an earlier export of the
        /// form; defaults to `template-<form>`
        #[arg(long, value_name = "PATH")]
        path: Option<PathBuf>,

        /// The GitHub repository the template will live in, as `owner/name`;
        /// over an earlier export, its origin
        #[arg(long, value_name = "OWNER/NAME")]
        repo: Option<String>,
    },
    /// Keep exported templates up with their form: write the form over each,
    /// push it, and tag it once its CI passes
    Follow {
        /// Clones of the template repositories
        #[arg(required = true, value_name = "PATH")]
        paths: Vec<PathBuf>,

        /// Say what would change, and write, push and tag nothing
        #[arg(long)]
        dry_run: bool,
    },
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
pub struct DoctorArgs {
    /// The repositories to examine; defaults to the one the current directory
    /// is in. More than one is reported as a table
    #[arg(value_name = "PATH", conflicts_with = "all")]
    pub paths: Vec<PathBuf>,

    /// Examine every project in rigger's record, as a table
    #[arg(long)]
    pub all: bool,

    /// Print one JSON document instead of the report
    #[arg(long)]
    pub json: bool,

    /// Write the repairs the findings carry, then examine again
    #[arg(long)]
    pub fix: bool,

    /// Say why a rule exists and how to satisfy it, instead of examining
    #[arg(long, value_name = "RULE", conflicts_with_all = ["paths", "all", "json", "fix"])]
    pub explain: Option<crate::commands::doctor::Rule>,
}

#[derive(clap::Args)]
pub struct CheckArgs {
    /// The repositories to check; defaults to the one the current directory
    /// is in. More than one is reported as a table
    #[arg(value_name = "PATH", conflicts_with = "all")]
    pub paths: Vec<PathBuf>,

    /// Check every project in rigger's record, as a table
    #[arg(long)]
    pub all: bool,

    /// Print one JSON document instead of the report
    #[arg(long)]
    pub json: bool,

    /// Check what a release put out as well: its page and files, the
    /// registries, the links, the Checked live list of its notes. The newest
    /// release unless a tag is named. Asks the forge and the registries
    #[arg(long, value_name = "TAG", num_args = 0..=1, default_missing_value = "")]
    pub release: Option<String>,

    /// With --release: install the release with the repository's own
    /// installer into a temporary directory, and prove the build is the
    /// release's own
    #[arg(long, requires = "release")]
    pub install: bool,

    /// Say why a check exists and how to satisfy it, instead of checking
    #[arg(long, value_name = "CHECK", conflicts_with_all = ["paths", "all", "json", "release"])]
    pub explain: Option<crate::commands::check::Shop>,
}

#[derive(clap::Args)]
pub struct RepoArgs {
    /// The repository; defaults to the one the current directory is in
    pub path: Option<PathBuf>,

    /// Say what would change on GitHub, and change nothing
    #[arg(long)]
    pub dry_run: bool,

    /// Create the repository private rather than public; one that exists keeps its visibility
    #[arg(long)]
    pub private: bool,
}

#[derive(clap::Args)]
pub struct OpenArgs {
    /// Which shopfront [default: repo]
    #[arg(value_name = "PLACE")]
    pub place: Option<crate::commands::open::Place>,

    /// The repository; defaults to the one the current directory is in
    #[arg(long, value_name = "PATH")]
    pub path: Option<PathBuf>,

    /// Print the address instead of opening it
    #[arg(long)]
    pub print: bool,
}

#[derive(clap::Args)]
pub struct CleanArgs {
    /// The repositories to clean; defaults to the one the current directory is in
    #[arg(value_name = "PATH", conflicts_with = "all")]
    pub paths: Vec<PathBuf>,

    /// Clean every project in rigger's record
    #[arg(long)]
    pub all: bool,

    /// Say what would be removed and what it weighs, and remove nothing
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Subcommand)]
pub enum BrandCommand {
    /// Codes for a product's name, and a code and colour held to the line's marks
    Pick(BrandPickArgs),
}

#[derive(clap::Args)]
pub struct BrandPickArgs {
    /// The product's name; defaults to the project the current directory is in
    pub name: Option<String>,

    /// The two-letter code to hold to the line, instead of listing candidates
    #[arg(long, value_name = "CODE")]
    pub code: Option<String>,

    /// The mark's colour, or the first of a pair, as `#rrggbb`
    #[arg(long, value_name = "HEX", alias = "color")]
    pub colour: Option<String>,

    /// The second colour of a pair, where the gradient runs to
    #[arg(long, value_name = "HEX")]
    pub to: Option<String>,
}

#[derive(clap::Args)]
pub struct AssetsArgs {
    /// The repository; defaults to the one the current directory is in
    pub path: Option<PathBuf>,

    /// Take the masters in first from the mark's generator: a directory of
    /// `<mark>-L.svg`, `-M.svg`, `-S.svg` and `-banner.svg`
    #[arg(long, value_name = "DIR")]
    pub from: Option<PathBuf>,

    /// The mark's name in that directory; defaults to the project's
    #[arg(long, value_name = "NAME", requires = "from")]
    pub mark: Option<String>,
}

#[derive(clap::Args)]
pub struct CiArgs {
    /// Only these jobs, by their id in the workflow; defaults to every job
    /// the push would start
    #[arg(value_name = "JOB")]
    pub jobs: Vec<String>,

    /// The repository; defaults to the one the current directory is in
    #[arg(long, value_name = "PATH")]
    pub path: Option<PathBuf>,

    /// Say what would run and what would not, and run nothing
    #[arg(long)]
    pub list: bool,
}

/// A command that reads one repository.
#[derive(clap::Args)]
pub struct PathArgs {
    /// The repository; defaults to the one the current directory is in
    pub path: Option<PathBuf>,
}

#[derive(clap::Args)]
pub struct UpgradeArgs {
    /// The repository to upgrade; defaults to the current directory
    pub path: Option<PathBuf>,

    /// Show what would change, with the diffs, and write nothing
    #[arg(long)]
    pub dry_run: bool,

    /// Write without asking first
    #[arg(short = 'y', long = "yes")]
    pub assume_yes: bool,
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
    /// The shape of the project [default: spa]
    #[arg(long, value_name = "FORM")]
    pub form: Option<Form>,

    /// Generate from a template instead of a built-in form: a path
    /// (`./mine`), a name under ~/.lyrn/templates, or `owner/repo@tag`
    #[arg(long, value_name = "SOURCE", conflicts_with = "form")]
    pub template: Option<String>,

    /// The application a plugin extends (see `lyrn forms`); `--form plugin` only
    #[arg(long, value_name = "HOST")]
    pub host: Option<String>,

    #[command(flatten)]
    pub identity: IdentityArgs,

    /// Optional pieces to generate with, comma-separated (see `lyrn forms`)
    #[arg(long = "with", value_name = "ADDON", value_delimiter = ',')]
    pub with: Vec<String>,

    /// Answer a cargo-generate template's placeholder, as `key=value`
    #[arg(long, value_name = "KEY=VALUE")]
    pub define: Vec<String>,

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
#[derive(clap::Args, Default, Clone)]
pub struct IdentityArgs {
    /// The product's colour: a product of the line, or a `#rrggbb` value
    #[arg(long, value_name = "COLOUR")]
    pub accent: Option<String>,

    /// One line describing what the project is
    #[arg(long)]
    pub description: Option<String>,

    /// The author recorded in LICENSE; defaults to the global git identity
    #[arg(long)]
    pub author: Option<String>,

    /// The GitHub repository it will live in, as `owner/name`
    #[arg(long, value_name = "OWNER/NAME")]
    pub repo: Option<String>,
}
