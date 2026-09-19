//! The `tauri-plugin` form: a Tauri 2 plugin, both halves of it.
//!
//! Four desktop products of the line already depend on Tauri plugins, so the
//! form generates one in the shape those are written: a Rust crate registered
//! on the application's builder, and an npm package the webview calls.
//!
//! What makes it a form rather than two projects is the agreement between the
//! halves. The plugin name, the command key and the permission generated from
//! it are three spellings of one thing, and a disagreement is not a build
//! failure - it is a permission error in the webview of whoever installed the
//! plugin. The templates declare each once and both sides' tests hold them.

use crate::generate::{Contents, SourceFile};
use crate::templates::cli;

/// The manifest text shipped with this form.
pub const MANIFEST: &str = include_str!("tauri-plugin/template.toml");

/// The files this form takes from `cli` unchanged.
const SHARED_WITH_CLI: &[&str] = &["LICENSE", "CHANGELOG.md", "cliff.toml", ".editorconfig", ".gitattributes"];

/// Every file the `tauri-plugin` form writes.
pub fn sources() -> Vec<SourceFile> {
    let mut files: Vec<SourceFile> = cli::sources().into_iter().filter(|f| SHARED_WITH_CLI.contains(&f.path)).collect();

    files.extend([
        SourceFile {
            path: "Cargo.toml",
            contents: Contents::Text(include_str!("tauri-plugin/Cargo.toml.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "build.rs",
            contents: Contents::Text(include_str!("tauri-plugin/build.rs.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "rustfmt.toml",
            contents: Contents::Text(include_str!("tauri-plugin/rustfmt.toml.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "src/lib.rs",
            contents: Contents::Text(include_str!("tauri-plugin/src/lib.rs.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "src/commands.rs",
            contents: Contents::Text(include_str!("tauri-plugin/src/commands.rs.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "src/error.rs",
            contents: Contents::Text(include_str!("tauri-plugin/src/error.rs.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "permissions/default.toml",
            contents: Contents::Text(include_str!("tauri-plugin/permissions-default.toml.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "guest-js/index.ts",
            contents: Contents::Text(include_str!("tauri-plugin/guest-js/index.ts.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "guest-js/index.test.ts",
            contents: Contents::Text(include_str!("tauri-plugin/guest-js/index.test.ts.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "package.json",
            contents: Contents::Text(include_str!("tauri-plugin/package.json.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "tsconfig.json",
            contents: Contents::Text(include_str!("tauri-plugin/tsconfig.json.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "README.md",
            contents: Contents::Text(include_str!("tauri-plugin/README.md.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: ".gitignore",
            contents: Contents::Text(include_str!("tauri-plugin/gitignore.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: ".github/workflows/ci.yml",
            contents: Contents::Text(include_str!("tauri-plugin/github/ci.yml.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: ".github/workflows/release.yml",
            contents: Contents::Text(include_str!("tauri-plugin/github/release.yml.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: ".github/workflows/publish.yml",
            contents: Contents::Text(include_str!("tauri-plugin/github/publish.yml.tmpl")),
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
            path: "docs/adr/0002-both-halves-ship-together.md",
            contents: Contents::Text(include_str!("tauri-plugin/docs-adr-0002.md.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "docs/adr/README.md",
            contents: Contents::Text(include_str!("tauri-plugin/docs-adr-README.md.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "lyrn.toml",
            contents: Contents::Text(include_str!("tauri-plugin/lyrn.toml.tmpl")),
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
        let parent: Vec<&str> = cli::sources().iter().map(|s| s.path).collect();
        for path in SHARED_WITH_CLI {
            assert!(parent.contains(path), "SHARED_WITH_CLI names `{path}`, which the cli form does not write");
        }
    }

    #[test]
    fn the_tauri_plugin_form_writes_no_file_twice() {
        let mut paths: Vec<&str> = sources().iter().map(|s| s.path).collect();
        let count = paths.len();
        paths.sort_unstable();
        paths.dedup();
        assert_eq!(paths.len(), count, "the tauri-plugin form writes some path more than once");
    }

    #[test]
    fn the_command_key_is_declared_in_every_place_that_has_to_agree() {
        // Three spellings of one thing: the list build.rs generates a
        // permission from, the set an application grants, and the string the
        // webview invokes. Any of them written out by hand would be right on
        // the day it was written and wrong at the first rename.
        let must_ask: &[&str] = &["build.rs", "permissions/default.toml", "guest-js/index.ts"];

        for path in must_ask {
            let source = sources()
                .into_iter()
                .find(|s| s.path == *path)
                .unwrap_or_else(|| panic!("the form no longer writes `{path}`"));
            let Contents::Text(text) = source.contents else {
                panic!("`{path}` is not text")
            };
            assert!(
                text.contains("{{ command_key }}"),
                "`{path}` no longer asks for the command key, so a rename would pass it by"
            );
        }
    }
}
