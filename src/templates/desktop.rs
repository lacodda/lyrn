//! The `desktop` form: Tauri 2 wrapped around the `spa` stack.
//!
//! A desktop app on this line is the SPA plus a Rust shell, so the form reuses
//! the spa template for everything that is genuinely the same file and
//! overrides the handful that differ - the manifest, the Vite config, the
//! entry point, CI. Copying all twenty-five would mean fixing every future
//! defect twice.

use crate::generate::{Contents, SourceFile};
use crate::model::Addon;
use crate::templates::spa;

/// The manifest text shipped with this form.
pub const MANIFEST: &str = include_str!("desktop/template.toml");

/// Files the desktop form provides itself, replacing the spa version.
const OVERRIDDEN: &[&str] = &[
    "package.json",
    "vite.config.ts",
    "src/main.tsx",
    "src/App.tsx",
    "src/App.test.tsx",
    "README.md",
    ".github/workflows/ci.yml",
    ".gitignore",
];

/// Every file the `desktop` form writes.
pub fn sources() -> Vec<SourceFile> {
    let mut files: Vec<SourceFile> = spa::sources().into_iter().filter(|f| !OVERRIDDEN.contains(&f.path)).collect();

    files.extend([
        SourceFile {
            path: "package.json",
            contents: Contents::Text(include_str!("desktop/package.json.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "vite.config.ts",
            contents: Contents::Text(include_str!("desktop/vite.config.ts.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "src/main.tsx",
            contents: Contents::Text(include_str!("desktop/src/main.tsx.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "src/App.tsx",
            contents: Contents::Text(include_str!("desktop/src/App.tsx.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "src/App.test.tsx",
            contents: Contents::Text(include_str!("desktop/src/App.test.tsx.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "README.md",
            contents: Contents::Text(include_str!("desktop/README.md.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: ".github/workflows/ci.yml",
            contents: Contents::Text(include_str!("desktop/github/ci.yml.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: ".gitignore",
            contents: Contents::Text(include_str!("desktop/gitignore.tmpl")),
            executable: false,
            addon: None,
        },
        // The Rust shell.
        SourceFile {
            path: "src-tauri/Cargo.toml",
            contents: Contents::Text(include_str!("desktop/src-tauri/Cargo.toml.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "src-tauri/build.rs",
            contents: Contents::Text(include_str!("desktop/src-tauri/build.rs.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "src-tauri/tauri.conf.json",
            contents: Contents::Text(include_str!("desktop/src-tauri/tauri.conf.json.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "src-tauri/capabilities/default.json",
            contents: Contents::Text(include_str!("desktop/src-tauri/capabilities.json.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "src-tauri/src/main.rs",
            contents: Contents::Text(include_str!("desktop/src-tauri/main.rs.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "src-tauri/src/lib.rs",
            contents: Contents::Text(include_str!("desktop/src-tauri/lib.rs.tmpl")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "src-tauri/src/commands.rs",
            contents: Contents::Text(include_str!("desktop/src-tauri/commands.rs.tmpl")),
            executable: false,
            addon: None,
        },
        // A Tauri build on Windows fails outright without an .ico, so the form
        // ships a placeholder: a graphite tile with `??`, obviously not a real
        // mark. Deliberately plain - a pretty placeholder is one nobody
        // replaces. In the .ico the largest image comes first, because Windows
        // reads the first entry for the taskbar and the title bar.
        SourceFile {
            path: "src-tauri/icons/icon.png",
            contents: Contents::Binary(include_bytes!("desktop/icons/icon.png")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "src-tauri/icons/icon.ico",
            contents: Contents::Binary(include_bytes!("desktop/icons/icon.ico")),
            executable: false,
            addon: None,
        },
        SourceFile {
            path: "src-tauri/rustfmt.toml",
            contents: Contents::Text(include_str!("desktop/rustfmt.toml.tmpl")),
            executable: false,
            addon: None,
        },
        // Translation, when asked for.
        SourceFile {
            path: "src/i18n/index.ts",
            contents: Contents::Text(include_str!("desktop/i18n/index.ts.tmpl")),
            executable: false,
            addon: Some(Addon::I18n),
        },
        SourceFile {
            path: "src/i18n/locales/en.json",
            contents: Contents::Text(include_str!("desktop/i18n/locales/en.json.tmpl")),
            executable: false,
            addon: Some(Addon::I18n),
        },
        SourceFile {
            path: "src/i18n/locales/ru.json",
            contents: Contents::Text(include_str!("desktop/i18n/locales/ru.json.tmpl")),
            executable: false,
            addon: Some(Addon::I18n),
        },
        SourceFile {
            path: "tools/check-locales.mjs",
            contents: Contents::Text(include_str!("desktop/tools/check-locales.mjs.tmpl")),
            executable: false,
            addon: Some(Addon::I18n),
        },
    ]);

    files.sort_by(|a, b| a.path.cmp(b.path));
    files
}
