//! The `mono` form: a pnpm workspace that publishes a TypeScript package.
//!
//! The line has no standalone publishable TypeScript package - dowel, kjui and
//! lyrnui are all monorepos whose root is `private: true` and whose shipped
//! artefact sits under `packages/`. So the form is the monorepo, and the
//! package is what lives inside it.
//!
//! It shares the files of the standard with `spa` and nothing else: a
//! workspace root is not an application, so the manifests, the configs and the
//! workflows are its own.

use crate::generate::{Contents, SourceFile};
use crate::model::Addon;
use crate::templates::spa;

/// The manifest text shipped with this form.
pub const MANIFEST: &str = include_str!("mono/template.toml");

/// The files this form takes from `spa` unchanged.
///
/// A short, explicit list rather than a rule: what these have in common is
/// that they belong to the standard rather than to a stack, and that is a
/// judgement, not a path pattern. A test holds every entry to a file `spa`
/// actually writes, so the list cannot rot into silence.
const SHARED_WITH_SPA: &[&str] = &["LICENSE", "CHANGELOG.md", "cliff.toml", ".editorconfig", ".gitattributes"];

/// Every file the `mono` form writes.
pub fn sources() -> Vec<SourceFile> {
    let mut files: Vec<SourceFile> = spa::sources().into_iter().filter(|f| SHARED_WITH_SPA.contains(&f.path)).collect();

    files.extend([
        SourceFile {
            path: "package.json",
            contents: Contents::Text(include_str!("mono/package.json.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "pnpm-workspace.yaml",
            contents: Contents::Text(include_str!("mono/pnpm-workspace.yaml.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "tsconfig.base.json",
            contents: Contents::Text(include_str!("mono/tsconfig.base.json.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "eslint.config.js",
            contents: Contents::Text(include_str!("mono/eslint.config.js.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "vitest.config.ts",
            contents: Contents::Text(include_str!("mono/vitest.config.ts.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "tools/copy-readme.mjs",
            contents: Contents::Text(include_str!("mono/tools/copy-readme.mjs.tmpl")),
            executable: false,
            addon: None,
        },
        // The published package.
        SourceFile {
            path: "packages/{{ name }}/package.json",
            contents: Contents::Text(include_str!("mono/packages/package.json.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "packages/{{ name }}/tsconfig.json",
            contents: Contents::Text(include_str!("mono/packages/tsconfig.json.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "packages/{{ name }}/src/index.ts",
            contents: Contents::Text(include_str!("mono/packages/src/index.ts.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "packages/{{ name }}/src/greet.ts",
            contents: Contents::Text(include_str!("mono/packages/src/greet.ts.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "packages/{{ name }}/src/greet.test.ts",
            contents: Contents::Text(include_str!("mono/packages/src/greet.test.ts.tmpl")),
            executable: false,
            addon: None,
        },
        // The stand: a second workspace member, only when asked for.
        SourceFile {
            path: "stand/package.json",
            contents: Contents::Text(include_str!("mono/stand/package.json.tmpl")),
            executable: false,
            addon: Some(Addon::Stand),
        },
        SourceFile {
            path: "stand/tsconfig.json",
            contents: Contents::Text(include_str!("mono/stand/tsconfig.json.tmpl")),
            executable: false,
            addon: Some(Addon::Stand),
        },
        SourceFile {
            path: "stand/vite.config.ts",
            contents: Contents::Text(include_str!("mono/stand/vite.config.ts.tmpl")),
            executable: false,
            addon: Some(Addon::Stand),
        },
        SourceFile {
            path: "stand/index.html",
            contents: Contents::Text(include_str!("mono/stand/index.html.tmpl")),
            executable: false,
            addon: Some(Addon::Stand),
        },
        SourceFile {
            path: "stand/src/main.tsx",
            contents: Contents::Text(include_str!("mono/stand/src/main.tsx.tmpl")),
            executable: false,
            addon: Some(Addon::Stand),
        },
        SourceFile {
            path: "stand/src/App.tsx",
            contents: Contents::Text(include_str!("mono/stand/src/App.tsx.tmpl")),
            executable: false,
            addon: Some(Addon::Stand),
        },
        SourceFile {
            path: "stand/src/App.test.tsx",
            contents: Contents::Text(include_str!("mono/stand/src/App.test.tsx.tmpl")),
            executable: false,
            addon: Some(Addon::Stand),
        },
        // The rest of the standard, in this form's own shape.
        SourceFile {
            path: "README.md",
            contents: Contents::Text(include_str!("mono/README.md.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: ".gitignore",
            contents: Contents::Text(include_str!("mono/gitignore.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: ".github/workflows/ci.yml",
            contents: Contents::Text(include_str!("mono/github/ci.yml.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: ".github/workflows/release.yml",
            contents: Contents::Text(include_str!("mono/github/release.yml.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: ".github/workflows/publish.yml",
            contents: Contents::Text(include_str!("mono/github/publish.yml.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "docs/adr/0001-record-architecture-decisions.md",
            contents: Contents::Text(include_str!("spa/docs/0001-record-architecture-decisions.md.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "docs/adr/0002-the-package-is-built-by-tsc.md",
            contents: Contents::Text(include_str!("mono/docs-adr-0002.md.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "docs/adr/README.md",
            contents: Contents::Text(include_str!("mono/docs-adr-README.md.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "lyrn.toml",
            contents: Contents::Text(include_str!("spa/lyrn.toml.tmpl")),
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
    fn every_shared_entry_names_a_file_the_spa_form_writes() {
        // A stale entry stops sharing anything and says nothing: the file is
        // simply absent from the generated project, which is the standard
        // quietly getting smaller.
        let parent: Vec<&str> = spa::sources().iter().map(|s| s.path).collect();
        for path in SHARED_WITH_SPA {
            assert!(parent.contains(path), "SHARED_WITH_SPA names `{path}`, which the spa form does not write");
        }
    }

    #[test]
    fn the_mono_form_writes_no_file_twice() {
        let mut paths: Vec<&str> = sources().iter().map(|s| s.path).collect();
        let count = paths.len();
        paths.sort_unstable();
        paths.dedup();
        assert_eq!(paths.len(), count, "the mono form writes some path more than once");
    }
}
