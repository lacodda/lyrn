//! The `plugin` form: an executable that extends a host of the line.
//!
//! The line's plugin convention is a subprocess speaking JSON over stdio, not
//! a dynamic library and not a sandboxed runtime - the reasoning is in the ADR
//! the form writes. What the form has to get right is the shape the host reads:
//! the manifest's fields, the invocation, the outcome, and a test that runs the
//! binary the way the host runs it.
//!
//! Everything host-specific arrives through the context, from the table in
//! `crate::host`. The templates never name a host; they name `{{ host }}`.

use crate::generate::{Contents, SourceFile};
use crate::templates::cli;

/// The manifest text shipped with this form.
pub const MANIFEST: &str = include_str!("plugin/template.toml");

/// The files this form takes from `cli` unchanged.
///
/// A plugin is a Rust binary, so the parts of the standard that are about
/// being a Rust binary are the same file. What differs is everything about
/// being a plugin: the manifest, the sources, the tests, the README, and the
/// workflows that name the binary.
const SHARED_WITH_CLI: &[&str] = &["LICENSE", "CHANGELOG.md", "cliff.toml", ".editorconfig", ".gitattributes"];

/// Every file the `plugin` form writes.
pub fn sources() -> Vec<SourceFile> {
    let mut files: Vec<SourceFile> = cli::sources().into_iter().filter(|f| SHARED_WITH_CLI.contains(&f.path)).collect();

    files.extend([
        SourceFile {
            path: "Cargo.toml",
            contents: Contents::Text(include_str!("plugin/Cargo.toml.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "rustfmt.toml",
            contents: Contents::Text(include_str!("plugin/rustfmt.toml.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "src/main.rs",
            contents: Contents::Text(include_str!("plugin/src/main.rs.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "src/protocol.rs",
            contents: Contents::Text(include_str!("plugin/src/protocol.rs.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "tests/protocol.rs",
            contents: Contents::Text(include_str!("plugin/tests/protocol.rs.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "README.md",
            contents: Contents::Text(include_str!("plugin/README.md.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: ".gitignore",
            contents: Contents::Text(include_str!("plugin/gitignore.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: ".github/workflows/ci.yml",
            contents: Contents::Text(include_str!("plugin/github/ci.yml.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: ".github/workflows/release.yml",
            contents: Contents::Text(include_str!("plugin/github/release.yml.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "docs/adr/0001-record-architecture-decisions.md",
            contents: Contents::Text(include_str!("cli/docs-adr-0001.md.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "docs/adr/0002-the-plugin-is-a-subprocess.md",
            contents: Contents::Text(include_str!("plugin/docs-adr-0002.md.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "docs/adr/README.md",
            contents: Contents::Text(include_str!("plugin/docs-adr-README.md.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "lyrn.toml",
            contents: Contents::Text(include_str!("plugin/lyrn.toml.tmpl")),
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
    fn every_shared_entry_names_a_file_the_cli_form_writes() {
        // A stale entry stops sharing anything and says nothing: the file is
        // simply absent from the generated project, which is the standard
        // quietly getting smaller.
        let parent: Vec<&str> = cli::sources().iter().map(|s| s.path).collect();
        for path in SHARED_WITH_CLI {
            assert!(parent.contains(path), "SHARED_WITH_CLI names `{path}`, which the cli form does not write");
        }
    }

    #[test]
    fn the_plugin_form_writes_no_file_twice() {
        let mut paths: Vec<&str> = sources().iter().map(|s| s.path).collect();
        let count = paths.len();
        paths.sort_unstable();
        paths.dedup();
        assert_eq!(paths.len(), count, "the plugin form writes some path more than once");
    }

    #[test]
    fn the_form_names_no_host_of_its_own() {
        // Every host-specific word arrives through the context. A template
        // that spelled `kilna` would generate a plugin for kilna no matter
        // what `--host` said, and only the person running it would find out.
        for source in sources() {
            let Contents::Text(text) = source.contents else { continue };
            for host in crate::host::ALL.iter().map(|h| h.name).chain(crate::host::PLANNED.iter().copied()) {
                assert!(
                    !text.contains(host),
                    "{}: the template spells out the host `{host}` instead of asking for `{{{{ host }}}}`",
                    source.path
                );
            }
        }
    }
}
