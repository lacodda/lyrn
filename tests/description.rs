//! The description is the one free text a template pastes in, and it has to
//! survive every place it lands: a JSON value, a TOML value, a Rust literal,
//! an HTML attribute, JSX text, a block comment.
//!
//! Before, it went in bare. An apostrophe or a colon is ordinary text and a
//! broken file: `"description": "{{ description }}"` with a quote inside is no
//! longer JSON, and the project fails `pnpm install` on the day it is made.

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;

mod common;
use common::FORMS;

/// Everything that has broken a file somewhere: both quotes, a backslash, the
/// HTML and JSX specials, a TOML-looking colon, the end of a block comment -
/// and a newline, which ends a `///` line and starts a syntax error.
const HOSTILE: &str = "He said \"hi\" & <b>{x}</b> \\ it's: done */\nfor real";

/// The same text as the generator records it: one line.
const ONE_LINE: &str = "He said \"hi\" & <b>{x}</b> \\ it's: done */ for real";

fn generate(form: &str, root: &Path) {
    let mut cmd = Command::cargo_bin("lyrn").unwrap();
    cmd.args(["new", "demo-tool", "--form", form, "--yes", "--no-hooks", "--repo", "owner/demo-tool"])
        .args(["--description", HOSTILE]);
    if form == "plugin" {
        cmd.args(["--host", "kilna"]);
    }
    let with: Vec<&str> = common::combinations(form).into_iter().max_by_key(Vec::len).unwrap_or_default();
    if !with.is_empty() {
        cmd.arg("--with").arg(with.join(","));
    }
    if form == "docs" {
        fs::create_dir_all(root).unwrap();
        cmd.current_dir(root);
    } else {
        cmd.arg("--path").arg(root);
    }
    cmd.assert().success();
}

fn walk(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() { stack.push(path) } else { found.push(path) }
        }
    }
    found
}

/// Every JSON and TOML file of every form, with every add-on, still parses -
/// and where it carries the description, carries it word for word.
#[test]
fn every_structured_file_survives_the_description() {
    for form in FORMS {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("demo-tool");
        generate(form, &root);

        for path in walk(&root) {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            let text = fs::read_to_string(&path).unwrap_or_default();
            let shown = path.strip_prefix(&root).unwrap().display().to_string();

            if name.ends_with(".json") || name.ends_with(".webmanifest") {
                // tsconfig allows comments and trailing commas; nothing lyrn
                // writes there comes from the description.
                if name.starts_with("tsconfig") {
                    continue;
                }
                let value: serde_json::Value = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{form}: {shown} is not JSON any more: {e}"));
                // Other JSON has descriptions of its own - a Tauri capability
                // says what it grants - so only the product's files are read.
                let describes_the_product = name == "package.json" || name == "manifest.webmanifest";
                if let Some(description) = value.get("description").and_then(|d| d.as_str()).filter(|_| describes_the_product) {
                    assert_eq!(description, ONE_LINE, "{form}: {shown} garbles the description");
                }
            }
            if name.ends_with(".toml") && name != "cliff.toml" {
                let value: toml::Table = toml::from_str(&text).unwrap_or_else(|e| panic!("{form}: {shown} is not TOML any more: {e}"));
                let package = value.get("package").and_then(|p| p.as_table());
                // `filter` rather than a let-chain: those are stable from 1.88
                // and this crate promises 1.85. The library crate of a
                // workspace describes itself in terms of the project.
                let description = package
                    .and_then(|p| p.get("description"))
                    .and_then(|d| d.as_str())
                    .filter(|d| !d.starts_with("Core library"));
                if let Some(description) = description {
                    assert_eq!(description, ONE_LINE, "{form}: {shown} garbles the description");
                }
            }
        }
    }
}

/// The places that are code rather than data, checked by what they must say.
#[test]
fn code_carries_the_description_escaped_for_its_language() {
    let dir = tempfile::tempdir().unwrap();
    let spa = dir.path().join("spa");
    generate("spa", &spa);
    let html = fs::read_to_string(spa.join("index.html")).unwrap();
    assert!(html.contains("content=\"He said &quot;hi&quot; &amp; &lt;b&gt;"), "{html}");
    let home = fs::read_to_string(spa.join("src/pages/Home.tsx")).unwrap();
    // `{x}` in JSX text would be an expression; the entities are text.
    assert!(home.contains("&#123;x&#125;") && !home.contains("{x}"), "{home}");

    let service = dir.path().join("service");
    generate("service", &service);
    let config = fs::read_to_string(service.join("src/config.rs")).unwrap();
    assert!(
        config.contains(r#"about = "He said \"hi\" & <b>{x}</b> \\ it's: done */ for real""#),
        "{config}"
    );

    let plugin = dir.path().join("plugin");
    generate("plugin", &plugin);
    let tauri = dir.path().join("tauri-plugin");
    generate("tauri-plugin", &tauri);
    let guest = fs::read_to_string(tauri.join("guest-js/index.ts")).unwrap();
    // One `*/` would close the doc comment and leave the rest as code.
    assert_eq!(guest.matches("*/").count(), guest.matches("/**").count(), "{guest}");
}
