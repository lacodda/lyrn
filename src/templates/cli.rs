//! The `cli` form: a Rust command-line tool on the line's stack.
//!
//! The four CLIs on the line agree on clap, anyhow and dialoguer and disagree
//! about everything else, so that trio is what the form writes by default.
//! Secrets in the keyring and a `self-update` command are add-ons: half the
//! line has each, and generating both every time would ship dead code and drag
//! in dependencies most projects never call.

use crate::generate::SourceFile;
use crate::model::Addon;

/// The manifest text shipped with this form.
pub const MANIFEST: &str = include_str!("cli/template.toml");

/// Every file the `cli` form writes.
pub fn sources() -> Vec<SourceFile> {
    vec![
        SourceFile {
            path: "Cargo.toml",
            contents: include_str!("cli/Cargo.toml.tmpl"),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "build.rs",
            contents: include_str!("cli/build.rs.tmpl"),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "rustfmt.toml",
            contents: include_str!("cli/rustfmt.toml.tmpl"),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "src/main.rs",
            contents: include_str!("cli/src/main.rs.tmpl"),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "src/cli.rs",
            contents: include_str!("cli/src/cli.rs.tmpl"),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "src/commands/mod.rs",
            contents: include_str!("cli/src/commands_mod.rs.tmpl"),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "src/commands/hello.rs",
            contents: include_str!("cli/src/hello.rs.tmpl"),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "src/commands/secret.rs",
            contents: include_str!("cli/src/secret.rs.tmpl"),
            executable: false,
            addon: Some(Addon::Keyring),
        },
        SourceFile {
            path: "src/commands/self_update.rs",
            contents: include_str!("cli/src/self_update.rs.tmpl"),
            executable: false,
            addon: Some(Addon::SelfUpdate),
        },
        SourceFile {
            path: "src/update.rs",
            contents: include_str!("cli/src/update.rs.tmpl"),
            executable: false,
            addon: Some(Addon::SelfUpdate),
        },
        SourceFile {
            path: "tests/cli.rs",
            contents: include_str!("cli/tests/cli.rs.tmpl"),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "README.md",
            contents: include_str!("cli/README.md.tmpl"),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "LICENSE",
            contents: include_str!("cli/LICENSE.tmpl"),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "CHANGELOG.md",
            contents: include_str!("cli/CHANGELOG.md.tmpl"),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "cliff.toml",
            contents: include_str!("cli/cliff.toml.tmpl"),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: ".gitignore",
            contents: include_str!("cli/gitignore.tmpl"),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: ".editorconfig",
            contents: include_str!("cli/editorconfig.tmpl"),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: ".gitattributes",
            contents: include_str!("cli/gitattributes.tmpl"),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: ".github/workflows/ci.yml",
            contents: include_str!("cli/github/ci.yml.tmpl"),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: ".github/workflows/release.yml",
            contents: include_str!("cli/github/release.yml.tmpl"),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: ".github/workflows/publish.yml",
            contents: include_str!("cli/github/publish.yml.tmpl"),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "tools/install.sh",
            contents: include_str!("cli/tools/install.sh.tmpl"),
            executable: true,
            addon: None,
        },
        SourceFile {
            path: "tools/install.ps1",
            contents: include_str!("cli/tools/install.ps1.tmpl"),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "npm/package.json",
            contents: include_str!("cli/npm/package.json.tmpl"),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "npm/download.js",
            contents: include_str!("cli/npm/download.js.tmpl"),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "npm/install.js",
            contents: include_str!("cli/npm/install.js.tmpl"),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "npm/run.js",
            contents: include_str!("cli/npm/run.js.tmpl"),
            executable: true,
            addon: None,
        },
        SourceFile {
            path: "npm/prepack.js",
            contents: include_str!("cli/npm/prepack.js.tmpl"),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "docs/adr/0001-record-architecture-decisions.md",
            contents: include_str!("cli/docs-adr-0001.md.tmpl"),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "docs/adr/README.md",
            contents: include_str!("cli/docs-adr-README.md.tmpl"),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "lyrn.toml",
            contents: include_str!("cli/lyrn.toml.tmpl"),
            executable: false,
            addon: None,
        },
    ]
}
