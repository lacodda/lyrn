//! The `spa` form: Vite, React, TypeScript, Tailwind and the dowel theme,
//! plus the files the line's standard asks every repository to carry.
//!
//! Versions are pinned to what the line actually runs today, not to the
//! newest thing published: TypeScript stays on 6 because typescript-eslint 8
//! does not support 7, and pnpm stays on 10 because 11 is broken on Windows.

use crate::generate::SourceFile;

/// The manifest text shipped with this form.
pub const MANIFEST: &str = include_str!("spa/template.toml");

/// Every file the `spa` form writes.
pub fn sources() -> Vec<SourceFile> {
    vec![
        SourceFile {
            path: "package.json",
            contents: include_str!("spa/package.json.tmpl"),
            executable: false,
        },
        SourceFile {
            path: "pnpm-workspace.yaml",
            contents: include_str!("spa/pnpm-workspace.yaml.tmpl"),
            executable: false,
        },
        SourceFile {
            path: "vite.config.ts",
            contents: include_str!("spa/vite.config.ts.tmpl"),
            executable: false,
        },
        SourceFile {
            path: "vitest.config.ts",
            contents: include_str!("spa/vitest.config.ts.tmpl"),
            executable: false,
        },
        SourceFile {
            path: "tsconfig.json",
            contents: include_str!("spa/tsconfig.json.tmpl"),
            executable: false,
        },
        SourceFile {
            path: "eslint.config.js",
            contents: include_str!("spa/eslint.config.js.tmpl"),
            executable: false,
        },
        SourceFile {
            path: "index.html",
            contents: include_str!("spa/index.html.tmpl"),
            executable: false,
        },
        SourceFile {
            path: "src/main.tsx",
            contents: include_str!("spa/src/main.tsx.tmpl"),
            executable: false,
        },
        SourceFile {
            path: "src/App.tsx",
            contents: include_str!("spa/src/App.tsx.tmpl"),
            executable: false,
        },
        SourceFile {
            path: "src/App.test.tsx",
            contents: include_str!("spa/src/App.test.tsx.tmpl"),
            executable: false,
        },
        SourceFile {
            path: "src/styles.css",
            contents: include_str!("spa/src/styles.css.tmpl"),
            executable: false,
        },
        SourceFile {
            path: "src/lib/utils.ts",
            contents: include_str!("spa/src/lib/utils.ts.tmpl"),
            executable: false,
        },
        SourceFile {
            path: "src/vite-env.d.ts",
            contents: include_str!("spa/src/vite-env.d.ts.tmpl"),
            executable: false,
        },
        SourceFile {
            path: "components.json",
            contents: include_str!("spa/components.json.tmpl"),
            executable: false,
        },
        SourceFile {
            path: "README.md",
            contents: include_str!("spa/README.md.tmpl"),
            executable: false,
        },
        SourceFile {
            path: "LICENSE",
            contents: include_str!("spa/LICENSE.tmpl"),
            executable: false,
        },
        SourceFile {
            path: "CHANGELOG.md",
            contents: include_str!("spa/CHANGELOG.md.tmpl"),
            executable: false,
        },
        SourceFile {
            path: "cliff.toml",
            contents: include_str!("spa/cliff.toml.tmpl"),
            executable: false,
        },
        SourceFile {
            path: ".gitignore",
            contents: include_str!("spa/gitignore.tmpl"),
            executable: false,
        },
        SourceFile {
            path: ".editorconfig",
            contents: include_str!("spa/editorconfig.tmpl"),
            executable: false,
        },
        SourceFile {
            path: ".gitattributes",
            contents: include_str!("spa/gitattributes.tmpl"),
            executable: false,
        },
        SourceFile {
            path: ".github/workflows/ci.yml",
            contents: include_str!("spa/github/ci.yml.tmpl"),
            executable: false,
        },
        SourceFile {
            path: "docs/adr/0001-record-architecture-decisions.md",
            contents: include_str!("spa/docs/0001-record-architecture-decisions.md.tmpl"),
            executable: false,
        },
        SourceFile {
            path: "docs/adr/README.md",
            contents: include_str!("spa/docs/adr-README.md.tmpl"),
            executable: false,
        },
        SourceFile {
            path: "lyrn.toml",
            contents: include_str!("spa/lyrn.toml.tmpl"),
            executable: false,
        },
    ]
}
