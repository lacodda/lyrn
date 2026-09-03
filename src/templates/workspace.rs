//! The `workspace` form: a library crate and the CLI that uses it.
//!
//! Everything the `cli` form writes at the repository root that is genuinely
//! the same file - the installers, the npm wrapper, cliff, the editor config -
//! is the same file here, so the form reuses it rather than keeping a second
//! copy that would have to be fixed twice. What it replaces is what actually
//! differs: the manifests, the Rust sources that now live under `crates/`, and
//! the two workflows that have to name a package rather than assume there is
//! only one.

use crate::generate::{Contents, SourceFile};
use crate::model::Addon;
use crate::templates::cli;

/// The manifest text shipped with this form.
pub const MANIFEST: &str = include_str!("workspace/template.toml");

/// Files the cli form writes at the root that the workspace provides itself.
const OVERRIDDEN: &[&str] = &[
    "Cargo.toml",
    "README.md",
    ".github/workflows/ci.yml",
    ".github/workflows/publish.yml",
    "docs/adr/README.md",
];

/// Whether a cli file is Rust that belongs inside a crate rather than at the
/// repository root.
///
/// A rule rather than a list: a file added to the cli form later lands in the
/// crate by itself. A list would keep working and quietly write the new file
/// to the root, where cargo would not compile it and nothing would say so.
fn belongs_in_a_crate(path: &str) -> bool {
    path == "build.rs" || path.starts_with("src/") || path.starts_with("tests/")
}

/// Every file the `workspace` form writes.
pub fn sources() -> Vec<SourceFile> {
    let mut files: Vec<SourceFile> = cli::sources()
        .into_iter()
        .filter(|f| !OVERRIDDEN.contains(&f.path) && !belongs_in_a_crate(f.path))
        .collect();

    // The parts of the cli binary the workspace keeps unchanged, only deeper:
    // the command surface and the two add-ons are the same code whether or not
    // the logic behind them sits in a sibling crate.
    files.extend([
        SourceFile {
            path: "crates/{{ name }}/src/cli.rs",
            contents: Contents::Text(include_str!("cli/src/cli.rs.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "crates/{{ name }}/src/commands/mod.rs",
            contents: Contents::Text(include_str!("cli/src/commands_mod.rs.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "crates/{{ name }}/src/commands/secret.rs",
            contents: Contents::Text(include_str!("cli/src/secret.rs.tmpl")),
            executable: false,
            addon: Some(Addon::Keyring),
        },
        SourceFile {
            path: "crates/{{ name }}/src/commands/self_update.rs",
            contents: Contents::Text(include_str!("cli/src/self_update.rs.tmpl")),
            executable: false,
            addon: Some(Addon::SelfUpdate),
        },
        SourceFile {
            path: "crates/{{ name }}/src/update.rs",
            contents: Contents::Text(include_str!("cli/src/update.rs.tmpl")),
            executable: false,
            addon: Some(Addon::SelfUpdate),
        },
    ]);

    // What the workspace shape changes.
    files.extend([
        SourceFile {
            path: "Cargo.toml",
            contents: Contents::Text(include_str!("workspace/Cargo.toml.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "crates/{{ name }}-core/Cargo.toml",
            contents: Contents::Text(include_str!("workspace/crates/core/Cargo.toml.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "crates/{{ name }}-core/src/lib.rs",
            contents: Contents::Text(include_str!("workspace/crates/core/src/lib.rs.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "crates/{{ name }}-core/src/error.rs",
            contents: Contents::Text(include_str!("workspace/crates/core/src/error.rs.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "crates/{{ name }}/Cargo.toml",
            contents: Contents::Text(include_str!("workspace/crates/cli/Cargo.toml.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "crates/{{ name }}/build.rs",
            contents: Contents::Text(include_str!("workspace/crates/cli/build.rs.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "crates/{{ name }}/src/main.rs",
            contents: Contents::Text(include_str!("workspace/crates/cli/src/main.rs.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "crates/{{ name }}/src/commands/hello.rs",
            contents: Contents::Text(include_str!("workspace/crates/cli/src/hello.rs.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "crates/{{ name }}/tests/cli.rs",
            contents: Contents::Text(include_str!("workspace/crates/cli/tests/cli.rs.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "README.md",
            contents: Contents::Text(include_str!("workspace/README.md.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: ".github/workflows/ci.yml",
            contents: Contents::Text(include_str!("workspace/github/ci.yml.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: ".github/workflows/publish.yml",
            contents: Contents::Text(include_str!("workspace/github/publish.yml.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "docs/adr/0002-the-logic-lives-in-a-library-crate.md",
            contents: Contents::Text(include_str!("workspace/docs-adr-0002.md.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "docs/adr/README.md",
            contents: Contents::Text(include_str!("workspace/docs-adr-README.md.tmpl")),
            executable: false,
            addon: None,
        },
    ]);

    files
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_override_names_a_file_the_cli_form_writes() {
        // A stale entry stops protecting anything: the file it used to shadow
        // is gone, and the workspace's own version is written unconditionally
        // - which looks identical to working, until the parent moves a file
        // and the override silently shadows nothing.
        let parent: Vec<&str> = cli::sources().iter().map(|s| s.path).collect();
        for path in OVERRIDDEN {
            assert!(parent.contains(path), "OVERRIDDEN names `{path}`, which the cli form does not write");
        }
    }

    #[test]
    fn nothing_the_cli_form_writes_is_both_overridden_and_relocated() {
        // Saying both would be a contradiction the filter resolves by accident
        // rather than by decision.
        for path in OVERRIDDEN {
            assert!(!belongs_in_a_crate(path), "`{path}` is both overridden and relocated");
        }
    }

    #[test]
    fn the_workspace_writes_no_file_twice() {
        let mut paths: Vec<&str> = sources().iter().map(|s| s.path).collect();
        let count = paths.len();
        paths.sort_unstable();
        paths.dedup();
        assert_eq!(paths.len(), count, "the workspace form writes some path more than once");
    }
}
