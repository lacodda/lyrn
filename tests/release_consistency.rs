//! Guards the facts that must agree before a version is published.
//!
//! lyrn ships to three places - GitHub, crates.io and npm - each of which
//! renders its own copy of the metadata. They drift silently: nothing fails
//! when `npm/package.json` still says 1.3.0, or when the npm page describes
//! the product differently from the crate. The drift is only visible after
//! publishing, when it is too late to take back.
//!
//! These checks run in CI, so a mismatch fails the build instead of shipping.

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: impl AsRef<Path>) -> String {
    let path = repo_root().join(path);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

/// Extracts a top-level `key = "value"` from Cargo.toml.
///
/// Deliberately naive: it reads only the `[package]` block, which is all these
/// checks need, and avoids adding a TOML parser as a dev-dependency.
fn cargo_field(key: &str) -> String {
    let manifest = read("Cargo.toml");
    for line in manifest.lines() {
        let line = line.trim();
        // Stop at the next section: `version` also appears under [dependencies]
        // and in every dependency entry.
        if line.starts_with('[') && line != "[package]" {
            break;
        }
        let Some((name, value)) = line.split_once('=') else { continue };
        // Exact match, so `rust-version` cannot answer a lookup for `version`.
        if name.trim() != key {
            continue;
        }
        return value.trim().trim_matches('"').to_string();
    }
    panic!("`{key}` not found in the [package] block of Cargo.toml");
}

/// Extracts a `"key": "value"` from a JSON file, without a JSON dependency.
fn json_field(file: &str, key: &str) -> String {
    let text = read(file);
    let needle = format!("\"{key}\"");
    let start = text.find(&needle).unwrap_or_else(|| panic!("`{key}` not found in {file}"));
    let after = &text[start + needle.len()..];
    let after = after.trim_start().trim_start_matches(':').trim_start();
    let after = after.strip_prefix('"').unwrap_or_else(|| panic!("`{key}` in {file} is not a string"));
    after[..after.find('"').expect("unterminated string")].to_string()
}

#[test]
fn the_npm_package_version_matches_the_crate() {
    let crate_version = cargo_field("version");
    let npm_version = json_field("npm/package.json", "version");

    assert_eq!(
        npm_version, crate_version,
        "npm/package.json version ({npm_version}) differs from Cargo.toml ({crate_version}); \
         the npm page would advertise a version that was never released"
    );
}

#[test]
fn the_npm_wrapper_points_at_the_matching_release() {
    // `lyrn.binary` pins the tag the wrapper downloads. If it lags behind, npm
    // installs an older binary than the page advertises.
    let crate_version = cargo_field("version");
    let binary_tag = json_field("npm/package.json", "binary");

    assert_eq!(
        binary_tag,
        format!("v{crate_version}"),
        "npm/package.json pins the binary at {binary_tag}, but the crate is {crate_version}"
    );
}

#[test]
fn the_three_storefronts_describe_the_same_product() {
    let crate_description = cargo_field("description");
    let npm_description = json_field("npm/package.json", "description");

    assert_eq!(npm_description, crate_description, "the npm page and the crate describe lyrn differently");
}

#[test]
fn every_command_the_readme_shows_exists_in_the_cli() {
    // A README that documents a command the binary does not have is worse than
    // one that documents nothing: it is confidently wrong.
    let readme = read("README.md");
    let help = String::from_utf8(Command::cargo_bin("lyrn").unwrap().arg("--help").output().unwrap().stdout).unwrap();

    for line in readme.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("$ lyrn ") else { continue };
        let Some(command) = rest.split_whitespace().next() else { continue };
        // Placeholders and flags are not subcommands.
        if command.starts_with('-') || command.starts_with('<') {
            continue;
        }
        assert!(help.contains(command), "README shows `lyrn {command}`, which `lyrn --help` does not list");
    }
}

#[test]
fn every_command_the_docs_reference_exists_in_the_cli() {
    // The rule of the line: a command without a page does not exist. This is
    // the other direction - a page without a command.
    let help = String::from_utf8(Command::cargo_bin("lyrn").unwrap().arg("--help").output().unwrap().stdout).unwrap();
    let reference = repo_root().join("docs/src/content/docs/reference");

    for entry in fs::read_dir(&reference).expect("the reference directory is missing") {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|e| e != "md") {
            continue;
        }
        let stem = path.file_stem().unwrap().to_string_lossy().to_string();
        assert!(
            help.contains(&stem),
            "docs/reference/{stem}.md documents `lyrn {stem}`, which the CLI does not have"
        );
    }
}

#[test]
fn every_command_has_a_reference_page() {
    let help = String::from_utf8(Command::cargo_bin("lyrn").unwrap().arg("--help").output().unwrap().stdout).unwrap();
    let reference = repo_root().join("docs/src/content/docs/reference");

    // The subcommand list sits between the `Commands:` header and the next
    // blank line; `help` is clap's own and needs no page.
    let commands: Vec<String> = help
        .lines()
        .skip_while(|l| !l.starts_with("Commands:"))
        .skip(1)
        .take_while(|l| !l.trim().is_empty())
        .filter_map(|l| l.split_whitespace().next().map(str::to_string))
        .filter(|c| c != "help")
        .collect();

    assert!(!commands.is_empty(), "no commands parsed out of `lyrn --help`");

    for command in commands {
        let page = reference.join(format!("{command}.md"));
        assert!(page.is_file(), "`lyrn {command}` has no page at docs/reference/{command}.md");
    }
}

#[test]
fn the_docs_site_and_the_crate_describe_the_same_product() {
    let crate_description = cargo_field("description");
    let astro = read("docs/astro.config.mjs");

    // Starlight's description ends up in the site's meta tags, where a stale
    // sentence is invisible until someone shares a link.
    assert!(
        astro.contains(crate_description.trim_end_matches('.')),
        "docs/astro.config.mjs describes lyrn differently from Cargo.toml"
    );
}

#[test]
fn the_installers_agree_on_the_repository() {
    for installer in ["tools/install.sh", "tools/install.ps1"] {
        let text = read(installer);
        assert!(text.contains("lacodda/lyrn"), "{installer} does not point at lacodda/lyrn");
        // The tag must come from the redirect, never from the rate-limited API.
        assert!(
            !text.contains("api.github.com"),
            "{installer} asks api.github.com for the latest tag; unauthenticated calls are \
             capped at 60/hour per IP and the installer would fail for reasons of someone else's making"
        );
    }
}
