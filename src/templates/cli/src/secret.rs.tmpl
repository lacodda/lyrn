//! Secrets live in the OS keyring, never in a config file.
//!
//! A config file is copied into backups, synced to other machines, pasted into
//! issues and committed by accident. The keyring is the one place the operating
//! system already protects, so a secret that has to persist goes there and the
//! config keeps only its name.

use anyhow::{Context, Result};
use dialoguer::Password;

use crate::cli::SecretCommand;

const SERVICE: &str = "{{ name }}";

fn entry(key: &str) -> Result<keyring::Entry> {
    keyring::Entry::new(SERVICE, key).with_context(|| format!("opening the keyring entry for `{key}`"))
}

pub fn run(command: SecretCommand) -> Result<()> {
    match command {
        SecretCommand::Set { key } => {
            // Read from the terminal rather than an argument: an argument ends
            // up in the shell history and in the process list.
            let value = Password::new().with_prompt(format!("Secret for `{key}`")).interact()?;
            entry(&key)?.set_password(&value).with_context(|| format!("storing `{key}`"))?;
            println!("Stored `{key}`.");
        }
        SecretCommand::Get { key } => {
            let value = entry(&key)?.get_password().with_context(|| format!("reading `{key}`"))?;
            println!("{value}");
        }
        SecretCommand::Remove { key } => {
            entry(&key)?.delete_credential().with_context(|| format!("removing `{key}`"))?;
            println!("Removed `{key}`.");
        }
    }
    Ok(())
}
