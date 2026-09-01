//! The shapes a template is described with.
//!
//! A template is a directory plus a `template.toml` manifest. The manifest
//! declares the variables the files interpolate, the questions the wizard asks
//! when it has a terminal, and the hooks that run once the files are on disk.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// The form a generated project takes. One form, one shape of repository.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Form {
    /// Vite + React + TypeScript + Tailwind with the dowel theme.
    Spa,
    /// A Rust command-line tool: clap, anyhow, dialoguer.
    Cli,
}

impl Form {
    /// Every form the binary carries built in.
    pub const ALL: &'static [Form] = &[Form::Spa, Form::Cli];

    pub fn as_str(self) -> &'static str {
        match self {
            Form::Spa => "spa",
            Form::Cli => "cli",
        }
    }

    /// One line for `lyrn forms` and for the wizard.
    pub fn summary(self) -> &'static str {
        match self {
            Form::Spa => "Single-page app: Vite, React, TypeScript, Tailwind, dowel",
            Form::Cli => "Command-line tool: Rust, clap, anyhow, dialoguer",
        }
    }

    /// The add-ons this form understands.
    pub fn addons(self) -> &'static [Addon] {
        match self {
            Form::Spa => &[],
            Form::Cli => &[Addon::Keyring, Addon::SelfUpdate],
        }
    }
}

/// An optional piece a form can be generated with.
///
/// The line's four CLIs disagree about these: turnout and sefy keep secrets in
/// the OS keyring, kasl and turnout update themselves. Putting either in every
/// generated project would ship dead code and drag in dependencies - reqwest
/// and an archive reader for one, a platform keyring for the other - that most
/// projects never call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Addon {
    /// Secrets in the OS keyring, never in a config file.
    Keyring,
    /// `self-update`, checking the releases page for a newer version.
    SelfUpdate,
}

impl Addon {
    pub fn as_str(self) -> &'static str {
        match self {
            Addon::Keyring => "keyring",
            Addon::SelfUpdate => "self-update",
        }
    }

    pub fn summary(self) -> &'static str {
        match self {
            Addon::Keyring => "Secrets in the OS keyring, never in a config file",
            Addon::SelfUpdate => "A `self-update` command that reads the releases page",
        }
    }
}

impl std::fmt::Display for Addon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for Addon {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "keyring" => Ok(Addon::Keyring),
            "self-update" => Ok(Addon::SelfUpdate),
            other => Err(format!("unknown add-on `{other}` (known: keyring, self-update)")),
        }
    }
}

impl std::fmt::Display for Form {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for Form {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "spa" => Ok(Form::Spa),
            "cli" => Ok(Form::Cli),
            other => Err(format!(
                "unknown form `{other}` (known: {})",
                Form::ALL.iter().map(|f| f.as_str()).collect::<Vec<_>>().join(", ")
            )),
        }
    }
}

/// A template's manifest: `template.toml` at the root of the template.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TemplateManifest {
    /// Human-readable name, shown while generating.
    #[serde(default)]
    pub name: String,
    /// What the template produces, one line.
    #[serde(default)]
    pub description: String,
    /// The version of the line's standard the template writes.
    #[serde(default)]
    pub standard: String,
    /// Files copied verbatim: no placeholder is substituted inside them.
    ///
    /// Anything binary, or anything whose own syntax collides with the
    /// placeholder syntax, belongs here.
    #[serde(default)]
    pub verbatim: Vec<String>,
    /// Commands to run in the new project once the files are written.
    #[serde(default)]
    pub hooks: Vec<Hook>,
}

/// A command the template asks for after the files land.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hook {
    /// Shown to the user while it runs.
    pub name: String,
    /// The program and its arguments, already split.
    pub run: Vec<String>,
    /// Skip the hook when the tool it needs is missing, rather than failing.
    #[serde(default)]
    pub optional: bool,
}

/// The answers a generation runs with, and the source of every placeholder.
#[derive(Debug, Clone)]
pub struct Context {
    values: BTreeMap<String, String>,
}

impl Context {
    pub fn new() -> Self {
        Self { values: BTreeMap::new() }
    }

    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) -> &mut Self {
        self.values.insert(key.into(), value.into());
        self
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}
