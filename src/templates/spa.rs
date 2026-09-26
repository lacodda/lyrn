//! The `spa` form: Vite, React, TypeScript, Tailwind and the dowel theme,
//! plus the files the line's standard asks every repository to carry.
//!
//! Versions are pinned to what the line actually runs today, not to the
//! newest thing published: TypeScript stays on 6 because typescript-eslint 8
//! does not support 7, and pnpm stays on 10 because 11 is broken on Windows.

use crate::generate::{Contents, SourceFile};
use crate::model::Addon;
use crate::templates::dowel;

/// The manifest text shipped with this form.
pub const MANIFEST: &str = include_str!("spa/template.toml");

/// Every file the `spa` form writes.
pub fn sources() -> Vec<SourceFile> {
    vec![
        SourceFile {
            path: "package.json",
            contents: Contents::Text(include_str!("spa/package.json.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "pnpm-workspace.yaml",
            contents: Contents::Text(include_str!("spa/pnpm-workspace.yaml.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "vite.config.ts",
            contents: Contents::Text(include_str!("spa/vite.config.ts.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "vitest.config.ts",
            contents: Contents::Text(include_str!("spa/vitest.config.ts.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "tsconfig.json",
            contents: Contents::Text(include_str!("spa/tsconfig.json.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "eslint.config.js",
            contents: Contents::Text(include_str!("spa/eslint.config.js.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "index.html",
            contents: Contents::Text(include_str!("spa/index.html.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "src/main.tsx",
            contents: Contents::Text(include_str!("spa/src/main.tsx.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "src/App.tsx",
            contents: Contents::Text(include_str!("spa/src/App.tsx.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "src/App.test.tsx",
            contents: Contents::Text(include_str!("spa/src/App.test.tsx.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "src/pages/Home.tsx",
            contents: Contents::Text(include_str!("spa/src/pages/Home.tsx.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "src/pages/Home.test.tsx",
            contents: Contents::Text(include_str!("spa/src/pages/Home.test.tsx.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "src/lib/query.ts",
            contents: Contents::Text(include_str!("spa/src/lib/query.ts.tmpl")),
            executable: false,
            addon: Some(Addon::TanstackQuery),
        },
        SourceFile {
            path: "src/lib/query.test.ts",
            contents: Contents::Text(include_str!("spa/src/lib/query.test.ts.tmpl")),
            executable: false,
            addon: Some(Addon::TanstackQuery),
        },
        SourceFile {
            path: "src/lib/api.ts",
            contents: Contents::Text(include_str!("spa/src/lib/api.ts.tmpl")),
            executable: false,
            addon: Some(Addon::Auth),
        },
        SourceFile {
            path: "src/lib/session.tsx",
            contents: Contents::Text(include_str!("spa/src/lib/session.tsx.tmpl")),
            executable: false,
            addon: Some(Addon::Auth),
        },
        SourceFile {
            path: "src/pages/SignIn.tsx",
            contents: Contents::Text(include_str!("spa/src/pages/SignIn.tsx.tmpl")),
            executable: false,
            addon: Some(Addon::Auth),
        },
        SourceFile {
            path: "src/pages/SignIn.test.tsx",
            contents: Contents::Text(include_str!("spa/src/pages/SignIn.test.tsx.tmpl")),
            executable: false,
            addon: Some(Addon::Auth),
        },
        SourceFile {
            path: "src/test/session.tsx",
            contents: Contents::Text(include_str!("spa/src/test/session.tsx.tmpl")),
            executable: false,
            addon: Some(Addon::Auth),
        },
        // The sign-in screen is made of these; nothing else in a new project
        // is, so they come with the add-on and not before.
        dowel::FIELD.source(Some(Addon::Auth)),
        dowel::INPUT.source(Some(Addon::Auth)),
        dowel::PANEL.source(Some(Addon::Auth)),
        dowel::ALERT.source(Some(Addon::Auth)),
        SourceFile {
            path: "src/styles.css",
            contents: Contents::Text(include_str!("spa/src/styles.css.tmpl")),
            executable: false,
            addon: None,
        },
        // The one primitive the starting screen presses. A raw `<button>` is
        // refused by dowel's lint outside `components/ui/`, and a copy is how
        // every product of the line gets its Button.
        dowel::BUTTON.source(None),
        // The favicon: the line's umbrella mark at level S, until the product
        // has a mark of its own - a copy of dowel-ui's `lacodda-S.svg`, held
        // to the package by `tools/check-registry.mjs`.
        SourceFile {
            path: "public/favicon.svg",
            contents: Contents::Text(include_str!("dowel/marks/lacodda-S.svg")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "public/manifest.webmanifest",
            contents: Contents::Text(include_str!("spa/public/manifest.webmanifest.tmpl")),
            executable: false,
            addon: Some(Addon::Pwa),
        },
        SourceFile {
            path: "public/sw.js",
            contents: Contents::Text(include_str!("spa/public/sw.js.tmpl")),
            executable: false,
            addon: Some(Addon::Pwa),
        },
        // Drawn by `tools/render-placeholder-icon.py`, at the level their size
        // calls for - all plated, all above the S ceiling - and held to it by
        // `tests/placeholder_icon.rs`. The 512px one is the desktop form's.
        SourceFile {
            path: "public/icon-192.png",
            contents: Contents::Binary(include_bytes!("spa/public/icon-192.png")),
            executable: false,
            addon: Some(Addon::Pwa),
        },
        SourceFile {
            path: "public/icon-512.png",
            contents: Contents::Binary(include_bytes!("desktop/icons/icon.png")),
            executable: false,
            addon: Some(Addon::Pwa),
        },
        SourceFile {
            path: "public/apple-touch-icon.png",
            contents: Contents::Binary(include_bytes!("spa/public/apple-touch-icon.png")),
            executable: false,
            addon: Some(Addon::Pwa),
        },
        SourceFile {
            path: "src/pwa.ts",
            contents: Contents::Text(include_str!("spa/src/pwa.ts.tmpl")),
            executable: false,
            addon: Some(Addon::Pwa),
        },
        SourceFile {
            path: "src/pwa.test.ts",
            contents: Contents::Text(include_str!("spa/src/pwa.test.ts.tmpl")),
            executable: false,
            addon: Some(Addon::Pwa),
        },
        SourceFile {
            path: "tools/check-registry.mjs",
            contents: Contents::Text(include_str!("spa/tools/check-registry.mjs.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "src/lib/utils.ts",
            contents: Contents::Text(include_str!("spa/src/lib/utils.ts.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "src/vite-env.d.ts",
            contents: Contents::Text(include_str!("spa/src/vite-env.d.ts.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "components.json",
            contents: Contents::Text(include_str!("spa/components.json.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "README.md",
            contents: Contents::Text(include_str!("spa/README.md.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "LICENSE",
            contents: Contents::Text(include_str!("spa/LICENSE.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "CHANGELOG.md",
            contents: Contents::Text(include_str!("spa/CHANGELOG.md.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "cliff.toml",
            contents: Contents::Text(include_str!("spa/cliff.toml.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: ".gitignore",
            contents: Contents::Text(include_str!("spa/gitignore.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: ".editorconfig",
            contents: Contents::Text(include_str!("spa/editorconfig.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: ".gitattributes",
            contents: Contents::Text(include_str!("spa/gitattributes.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: ".github/workflows/ci.yml",
            contents: Contents::Text(include_str!("spa/github/ci.yml.tmpl")),
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
            path: "docs/adr/README.md",
            contents: Contents::Text(include_str!("spa/docs/adr-README.md.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "lyrn.toml",
            contents: Contents::Text(include_str!("spa/lyrn.toml.tmpl")),
            executable: false,
            addon: None,
        },
    ]
}
