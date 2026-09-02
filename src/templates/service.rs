//! The `service` form: an HTTP service on axum, sqlx and Postgres.
//!
//! Plain by default, which is what a service with no interface of its own
//! needs. `--with spa` adds the web UI, compiled into the binary so that a
//! self-hosted install is one file and the UI cannot be from a different build
//! than the API it calls.

use crate::generate::{Contents, SourceFile};
use crate::model::Addon;
use crate::templates::spa;

/// The manifest text shipped with this form.
pub const MANIFEST: &str = include_str!("service/template.toml");

/// Files the spa form writes that a service does not want at all: it is a Rust
/// project with a frontend inside it, not the other way round.
const NOT_FROM_SPA: &[&str] = &[
    "README.md",
    "LICENSE",
    "CHANGELOG.md",
    "cliff.toml",
    ".gitignore",
    ".editorconfig",
    ".gitattributes",
    ".github/workflows/ci.yml",
    "docs/adr/0001-record-architecture-decisions.md",
    "docs/adr/README.md",
    "lyrn.toml",
];

/// Every file the `service` form writes.
pub fn sources() -> Vec<SourceFile> {
    let mut files = vec![
        SourceFile {
            path: "Cargo.toml",
            contents: Contents::Text(include_str!("service/Cargo.toml.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "rustfmt.toml",
            contents: Contents::Text(include_str!("service/rustfmt.toml.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "src/main.rs",
            contents: Contents::Text(include_str!("service/src/main.rs.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "src/app.rs",
            contents: Contents::Text(include_str!("service/src/app.rs.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "src/config.rs",
            contents: Contents::Text(include_str!("service/src/config.rs.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "src/error.rs",
            contents: Contents::Text(include_str!("service/src/error.rs.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "src/example.rs",
            contents: Contents::Text(include_str!("service/src/example.rs.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "src/health.rs",
            contents: Contents::Text(include_str!("service/src/health.rs.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "src/web.rs",
            contents: Contents::Text(include_str!("service/src/web.rs.tmpl")),
            executable: false,
            addon: Some(Addon::Spa),
        },
        SourceFile {
            path: "migrations/0001_init.sql",
            contents: Contents::Text(include_str!("service/migrations/0001_init.sql.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "tests/api.rs",
            contents: Contents::Text(include_str!("service/tests/api.rs.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "Dockerfile",
            contents: Contents::Text(include_str!("service/Dockerfile.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "deploy/compose.yml",
            contents: Contents::Text(include_str!("service/deploy/compose.yml.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "deploy/env.example",
            contents: Contents::Text(include_str!("service/deploy/env.example.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "README.md",
            contents: Contents::Text(include_str!("service/README.md.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "LICENSE",
            contents: Contents::Text(include_str!("service/LICENSE.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "CHANGELOG.md",
            contents: Contents::Text(include_str!("service/CHANGELOG.md.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "cliff.toml",
            contents: Contents::Text(include_str!("service/cliff.toml.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: ".gitignore",
            contents: Contents::Text(include_str!("service/gitignore.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: ".editorconfig",
            contents: Contents::Text(include_str!("service/editorconfig.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: ".gitattributes",
            contents: Contents::Text(include_str!("service/gitattributes.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: ".github/workflows/ci.yml",
            contents: Contents::Text(include_str!("service/github/ci.yml.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "docs/adr/0001-record-architecture-decisions.md",
            contents: Contents::Text(include_str!("service/docs-adr-0001.md.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "docs/adr/README.md",
            contents: Contents::Text(include_str!("service/docs-adr-README.md.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "lyrn.toml",
            contents: Contents::Text(include_str!("service/lyrn.toml.tmpl")),
            executable: false,
            addon: None,
        },
    ];

    // The web UI, when asked for: the spa form's own files, moved under
    // `frontend/` and marked as belonging to the add-on. Reusing them rather
    // than copying keeps one description of what the line's frontend is.
    files.extend(spa::sources().into_iter().filter(|f| !NOT_FROM_SPA.contains(&f.path)).map(|f| {
        SourceFile {
            path: FRONTEND_PATHS
                .iter()
                .find(|(from, _)| *from == f.path)
                .map(|(_, to)| *to)
                .expect("every spa file needs a place under frontend/"),
            contents: f.contents,
            executable: f.executable,
            addon: Some(Addon::Spa),
        }
    }));

    files.sort_by(|a, b| a.path.cmp(b.path));
    files
}

/// Where each file the spa form writes lives inside a service.
///
/// Spelled out rather than computed: a path that silently landed in the wrong
/// place would only be noticed by whoever tried to build the frontend.
const FRONTEND_PATHS: &[(&str, &str)] = &[
    ("package.json", "frontend/package.json"),
    ("pnpm-workspace.yaml", "frontend/pnpm-workspace.yaml"),
    ("vite.config.ts", "frontend/vite.config.ts"),
    ("vitest.config.ts", "frontend/vitest.config.ts"),
    ("tsconfig.json", "frontend/tsconfig.json"),
    ("eslint.config.js", "frontend/eslint.config.js"),
    ("index.html", "frontend/index.html"),
    ("components.json", "frontend/components.json"),
    ("src/main.tsx", "frontend/src/main.tsx"),
    ("src/App.tsx", "frontend/src/App.tsx"),
    ("src/App.test.tsx", "frontend/src/App.test.tsx"),
    ("src/styles.css", "frontend/src/styles.css"),
    ("src/lib/utils.ts", "frontend/src/lib/utils.ts"),
    ("src/vite-env.d.ts", "frontend/src/vite-env.d.ts"),
];
