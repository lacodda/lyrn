use clap::{Parser, Subcommand};

/// {{ description }}
#[derive(Parser)]
#[command(name = "{{ name }}", version, about, propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Greet somebody - replace this with the first real command
    Hello(HelloArgs),
{{#keyring}}
    /// Manage secrets, stored in the OS keyring
    Secret {
        #[command(subcommand)]
        command: SecretCommand,
    },
{{/keyring}}
{{#self-update}}
    /// Update this tool to the latest release
    SelfUpdate,
{{/self-update}}
}

#[derive(clap::Args)]
pub struct HelloArgs {
    /// Who to greet
    #[arg(default_value = "world")]
    pub name: String,
}
{{#keyring}}

#[derive(Subcommand)]
pub enum SecretCommand {
    /// Store a secret under a name
    Set {
        /// The name the secret is filed under
        key: String,
    },
    /// Print a stored secret
    Get { key: String },
    /// Forget a stored secret
    Remove { key: String },
}
{{/keyring}}
