//! `readme`: one README for every registry, and its links work away from
//! GitHub.
//!
//! crates.io and npm each show a README of the package's own, rendered away
//! from the repository. A second README beside a published package is a
//! shopfront nobody updates when the first one changes; a link written
//! relative to the repository resolves on GitHub and nowhere else.

use regex::Regex;

use super::product::{Kind, Product};
use crate::commands::doctor::{Finding, Outcome, Repo, is_test_material};

const README: &str = "README.md";

pub fn check(repo: &Repo, product: &Product) -> Outcome {
    if !repo.has_file(README) {
        return Outcome::Skip("no README.md - `lyrn doctor` asks for the standard's files".to_string());
    }
    let mut findings = Vec::new();

    for manifest in product
        .manifests
        .iter()
        .filter(|m| m.publishable && !m.is_root() && m.name.as_deref().is_some_and(|n| product.owns(n)))
    {
        let dir = manifest.dir();
        let second = repo.files.iter().filter(|f| !is_test_material(f)).find(|f| {
            f.rsplit_once('/')
                .is_some_and(|(d, name)| d == dir && name.to_ascii_lowercase().starts_with("readme"))
        });
        let shown_by_cargo = manifest.kind == Kind::Cargo && manifest.readme.as_deref().is_some_and(|r| !points_at_root(dir, r));
        if let Some(second) = second {
            let fix = if manifest.kind == Kind::Package {
                "remove it from git and let the package's `prepack` copy the root README in"
            } else {
                "remove it, and point `readme` at the root README"
            };
            findings.push(Finding::fail(format!("{second} is a second README beside {}, which a registry shows", manifest.path)).fix(fix));
        } else if shown_by_cargo {
            findings.push(
                Finding::fail(format!(
                    "{} shows `{}` on crates.io, not the root README",
                    manifest.path,
                    manifest.readme.as_deref().unwrap_or_default()
                ))
                .fix(format!("`readme = \"{}\"`", relative_root(dir))),
            );
        }
    }

    let text = repo.read_text(README).unwrap_or_default();
    let mut links = 0;
    for (number, target) in links_in(&text) {
        links += 1;
        if !is_absolute(&target) {
            findings.push(
                Finding::fail(format!("README.md:{number}: `{target}` is relative, and breaks on crates.io and npm"))
                    .fix("link to the file on GitHub, or to its page on the documentation site"),
            );
        }
    }

    Outcome::from_findings(findings, || match links {
        0 => "one README, with no links".to_string(),
        1 => "one README, its link absolute".to_string(),
        n => format!("one README, its {n} links absolute"),
    })
}

/// Every link and image in a Markdown text, with its line, outside code.
fn links_in(text: &str) -> Vec<(usize, String)> {
    let markdown = Regex::new(r"!?\[[^\]]*\]\(\s*<?([^)\s>]*)").expect("a valid pattern");
    let html = Regex::new(r#"(?i)\b(?:src|href)\s*=\s*["']([^"']*)["']"#).expect("a valid pattern");
    let inline_code = Regex::new(r"`[^`]*`").expect("a valid pattern");
    let mut found = Vec::new();
    let mut fenced = false;
    for (i, line) in text.lines().enumerate() {
        if line.trim_start().starts_with("```") || line.trim_start().starts_with("~~~") {
            fenced = !fenced;
            continue;
        }
        if fenced {
            continue;
        }
        let line = inline_code.replace_all(line, "");
        for captures in markdown.captures_iter(&line).chain(html.captures_iter(&line)) {
            found.push((i + 1, captures[1].to_string()));
        }
    }
    found
}

fn is_absolute(target: &str) -> bool {
    target.starts_with("http://") || target.starts_with("https://") || target.starts_with("mailto:") || target.starts_with('#')
}

/// Whether a `readme` written in the package at `dir` names the root README.
fn points_at_root(dir: &str, readme: &str) -> bool {
    readme.trim_start_matches("./") == relative_root(dir)
}

fn relative_root(dir: &str) -> String {
    format!("{}{README}", "../".repeat(dir.split('/').count()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_are_found_outside_code() {
        let text = "[docs](https://x.dev) and ![logo](assets/logo.svg)\n<img src=\"docs/a.png\">\n`[not](a/link)`\n```\n[nor](this)\n```\n[anchor](#install)\n";
        let links: Vec<String> = links_in(text).into_iter().map(|(_, t)| t).collect();
        assert_eq!(links, ["https://x.dev", "assets/logo.svg", "docs/a.png", "#install"]);
        assert!(!is_absolute("assets/logo.svg"));
        assert!(is_absolute("#install"));
    }

    #[test]
    fn a_crate_s_readme_is_the_root_one_by_its_relative_path() {
        assert!(points_at_root("crates/core", "../../README.md"));
        assert!(!points_at_root("crates/core", "README.md"));
        assert_eq!(relative_root("npm"), "../README.md");
    }
}
