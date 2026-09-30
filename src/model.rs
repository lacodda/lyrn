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
    /// A desktop application: Tauri 2 around the spa stack.
    Desktop,
    /// An HTTP service: axum, sqlx and Postgres.
    Service,
    /// A Cargo workspace: a library crate and the CLI that uses it.
    Workspace,
    /// A pnpm monorepo publishing a TypeScript package.
    Mono,
    /// A subprocess plugin for a host application of the line.
    Plugin,
    /// A Tauri 2 plugin: a Rust crate and the npm package that calls it.
    TauriPlugin,
    /// A Starlight documentation site, added to a repository that has none.
    Docs,
}

/// Where a form's files go.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    /// A directory of its own, which must not exist or must be empty.
    NewDirectory,
    /// An existing repository, beside what is already there. Nothing present
    /// is overwritten: a single file in the way refuses the whole generation.
    IntoExisting,
}

impl Form {
    /// Every form the binary carries built in.
    pub const ALL: &'static [Form] = &[
        Form::Spa,
        Form::Cli,
        Form::Desktop,
        Form::Service,
        Form::Workspace,
        Form::Mono,
        Form::Plugin,
        Form::TauriPlugin,
        Form::Docs,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Form::Spa => "spa",
            Form::Cli => "cli",
            Form::Desktop => "desktop",
            Form::Service => "service",
            Form::Workspace => "workspace",
            Form::Mono => "mono",
            Form::Plugin => "plugin",
            Form::TauriPlugin => "tauri-plugin",
            Form::Docs => "docs",
        }
    }

    /// One line for `lyrn forms` and for the wizard.
    pub fn summary(self) -> &'static str {
        match self {
            Form::Spa => "Single-page app: Vite, React, TypeScript, Tailwind, dowel",
            Form::Cli => "Command-line tool: Rust, clap, anyhow, dialoguer",
            Form::Desktop => "Desktop app: Tauri 2 around the spa stack",
            Form::Service => "HTTP service: axum, sqlx, Postgres",
            Form::Workspace => "Cargo workspace: a library crate plus the CLI that uses it",
            Form::Mono => "pnpm monorepo publishing a TypeScript package to npm",
            Form::Plugin => "Plugin for a host of the line: an executable speaking JSON over stdio",
            Form::TauriPlugin => "Tauri 2 plugin: a Rust crate and the npm package that calls it",
            Form::Docs => "Documentation site added to an existing repository: Starlight, llms.txt",
        }
    }

    /// The add-ons this form understands.
    pub fn addons(self) -> &'static [Addon] {
        match self {
            // The pieces the line's web frontends reach for, each taken from
            // the products that already have it: react-router as kasl-server
            // and kilna route, TanStack Query as kilna fetches, a cookie
            // session as all four signed-in frontends keep one, and the
            // manifest and worker rhapsod and hilvan install with.
            Form::Spa => &[Addon::Router, Addon::TanstackQuery, Addon::Auth, Addon::Pwa],
            Form::Cli => &[Addon::Keyring, Addon::SelfUpdate],
            Form::Desktop => &[Addon::I18n],
            // `demo` as kasl-server's KASL_DEMO: made-up data for an empty
            // database, and a refusal for one that holds real data.
            Form::Service => &[Addon::Spa, Addon::Demo],
            // The workspace inherits the cli form's add-ons: they are the same
            // command-line tool, only with its logic moved into a library.
            Form::Workspace => &[Addon::Keyring, Addon::SelfUpdate],
            Form::Mono => &[Addon::Stand],
            // A plugin lives for the duration of one call: it has nothing to
            // keep in a keyring and nothing to update itself from.
            Form::Plugin => &[],
            Form::TauriPlugin => &[],
            Form::Docs => &[],
        }
    }

    /// Whether this form is generated against a host application.
    pub fn takes_a_host(self) -> bool {
        matches!(self, Form::Plugin)
    }

    /// Whether an existing repository can be of this form. A documentation
    /// site is something a repository has, not something it is: it is added
    /// with `lyrn new --form docs`.
    pub fn adoptable(self) -> bool {
        self != Form::Docs
    }

    /// Whether this form starts a repository or adds to one.
    pub fn placement(self) -> Placement {
        match self {
            Form::Docs => Placement::IntoExisting,
            _ => Placement::NewDirectory,
        }
    }
}

/// An optional piece a form can be generated with.
///
/// The line disagrees with itself about these, which is exactly why they are
/// optional: turnout and sefy keep secrets in the OS keyring, kasl and turnout
/// update themselves, kilna speaks two languages and nitid speaks one. Putting
/// any of them in every generated project would ship dead code and the
/// dependencies behind it - reqwest and an archive reader, a platform keyring,
/// a translation runtime - to projects that never call them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Addon {
    /// Secrets in the OS keyring, never in a config file.
    Keyring,
    /// `self-update`, checking the releases page for a newer version.
    SelfUpdate,
    /// i18next with a gate holding every locale to the source language.
    I18n,
    /// A web UI compiled into the binary, served by the same process.
    Spa,
    /// A Vite page inside the workspace where the package is seen running.
    Stand,
    /// Screens at addresses: react-router, with the unknown ones sent home.
    Router,
    /// Server state through TanStack Query, with the line's defaults.
    TanstackQuery,
    /// A cookie session: who is signed in, asked of the server, and a
    /// sign-in screen until someone is.
    Auth,
    /// Installable, and opening without a network: a manifest, the icons an
    /// install asks for, and a service worker.
    Pwa,
    /// Made-up data for a demonstration, behind a flag that refuses to touch
    /// a database holding real data.
    Demo,
}

impl Addon {
    /// Every add-on some form has.
    pub const ALL: &'static [Addon] = &[
        Addon::Keyring,
        Addon::SelfUpdate,
        Addon::I18n,
        Addon::Spa,
        Addon::Stand,
        Addon::Router,
        Addon::TanstackQuery,
        Addon::Auth,
        Addon::Pwa,
        Addon::Demo,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Addon::Keyring => "keyring",
            Addon::SelfUpdate => "self-update",
            Addon::I18n => "i18n",
            Addon::Spa => "spa",
            Addon::Stand => "stand",
            Addon::Router => "router",
            Addon::TanstackQuery => "tanstack-query",
            Addon::Auth => "auth",
            Addon::Pwa => "pwa",
            Addon::Demo => "demo",
        }
    }

    pub fn summary(self) -> &'static str {
        match self {
            Addon::Keyring => "Secrets in the OS keyring, never in a config file",
            Addon::SelfUpdate => "A `self-update` command that reads the releases page",
            Addon::I18n => "i18next, with a gate holding every locale to the source",
            Addon::Spa => "A web UI compiled into the binary and served by it",
            Addon::Stand => "A Vite page in the workspace where the package runs",
            Addon::Router => "Screens at addresses: react-router, unknown ones sent home",
            Addon::TanstackQuery => "Server state through TanStack Query, on the line's defaults",
            Addon::Auth => "A cookie session and a sign-in screen until someone signs in",
            Addon::Pwa => "Installable and offline: a manifest, its icons, a service worker",
            Addon::Demo => "Made-up data in an empty database; one with real data refuses it",
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
            "i18n" => Ok(Addon::I18n),
            "spa" => Ok(Addon::Spa),
            "stand" => Ok(Addon::Stand),
            "router" => Ok(Addon::Router),
            "tanstack-query" => Ok(Addon::TanstackQuery),
            "auth" => Ok(Addon::Auth),
            "pwa" => Ok(Addon::Pwa),
            "demo" => Ok(Addon::Demo),
            other => Err(format!(
                "unknown add-on `{other}` (known: {})",
                Addon::ALL.iter().map(|a| a.as_str()).collect::<Vec<_>>().join(", ")
            )),
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
            "desktop" => Ok(Form::Desktop),
            "service" => Ok(Form::Service),
            "workspace" => Ok(Form::Workspace),
            "mono" => Ok(Form::Mono),
            "plugin" => Ok(Form::Plugin),
            "tauri-plugin" => Ok(Form::TauriPlugin),
            "docs" => Ok(Form::Docs),
            other => Err(format!(
                "unknown form `{other}` (known: {})",
                Form::ALL.iter().map(|f| f.as_str()).collect::<Vec<_>>().join(", ")
            )),
        }
    }
}

/// A template's manifest: `template.toml` at the root of the template.
///
/// Unknown fields are refused rather than skipped. A template written for a
/// newer lyrn may use a field this one does not know, and skipping it would
/// generate something the template never meant - silently. Refusing says
/// which field, and `lyrn` says which version understands it.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
    /// The form a template from outside is a version of. Built-in forms know
    /// their own and leave it out; a template from outside has to say, since
    /// `lyrn doctor`, `adopt` and `upgrade` all reason in forms.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub form: Option<Form>,
    /// The oldest lyrn that understands this template, as `2.9.0`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lyrn: Option<String>,
    /// Files copied verbatim: no placeholder is substituted inside them.
    ///
    /// Anything binary, or anything whose own syntax collides with the
    /// placeholder syntax, belongs here.
    #[serde(default)]
    pub verbatim: Vec<String>,
    /// Files that need the executable bit. A checkout on Windows loses it, so
    /// a template from outside names them rather than relying on the disk.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub executable: Vec<String>,
    /// Commands to run in the new project once the files are written.
    #[serde(default)]
    pub hooks: Vec<Hook>,
    /// The optional pieces a template from outside offers, and the files
    /// that come only with each. Built-in forms declare theirs in code.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub addons: Vec<AddonManifest>,
}

/// One add-on as a template from outside declares it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AddonManifest {
    /// What `--with` calls it, and what `{{#name}}` sections are keyed by.
    pub name: String,
    /// One line for `lyrn forms` and for a refusal that lists what exists.
    #[serde(default)]
    pub summary: String,
    /// The files written only with this add-on, as paths in the template.
    #[serde(default)]
    pub files: Vec<String>,
}

/// A command the template asks for after the files land.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hook {
    /// Shown to the user while it runs.
    pub name: String,
    /// The program and its arguments, already split.
    pub run: Vec<String>,
    /// Skip the hook when the tool it needs is missing, rather than failing.
    #[serde(default)]
    pub optional: bool,
    /// The directory to run it in, relative to the project root. A form that
    /// adds a subproject to an existing repository installs that subproject,
    /// not the repository around it.
    #[serde(default)]
    pub dir: Option<String>,
    /// Part of starting the repository: `git init` and the first commit.
    /// Skipped when the project lands inside a work tree that already exists,
    /// whose history and next commit belong to its owner.
    #[serde(default)]
    pub starts_repository: bool,
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
