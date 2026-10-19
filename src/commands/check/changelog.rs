//! `changelog`: the changelog has a section for the version the manifests say.
//!
//! The number moves in the release commit, and so does its section: a bump
//! without one is a release whose notes say nothing, and the GitHub release
//! page is written from them.

use super::product::Product;
use crate::commands::doctor::{Finding, Outcome, Repo};

const CHANGELOG: &str = "CHANGELOG.md";

pub fn check(repo: &Repo, product: &Product) -> Outcome {
    let Some((_, version)) = product.version() else {
        return Outcome::Skip("no manifest carries a version".to_string());
    };
    if !repo.has_file(CHANGELOG) {
        return Outcome::Skip("no CHANGELOG.md - `lyrn doctor` asks for the standard's files".to_string());
    }
    let text = repo.read_text(CHANGELOG).unwrap_or_default();
    let sections = sections(&text);
    if sections.iter().any(|s| s == version) {
        return Outcome::Pass(format!("{CHANGELOG} has its section for {version}"));
    }
    let released = repo
        .git
        .as_ref()
        .and_then(|git| git.run(&["tag", "--list", "v[0-9]*"]))
        .is_some_and(|tags| tags.lines().any(|t| !t.trim().is_empty()));
    if !released && sections.is_empty() {
        return Outcome::Pass("nothing released yet, and no section claims otherwise".to_string());
    }
    Outcome::Found(vec![
        Finding::fail(format!("{CHANGELOG} has no section for {version}, the version the manifests say")).fix(format!(
            "write it in the commit that moves the number: `git cliff --unreleased --tag v{version} --prepend {CHANGELOG}`"
        )),
    ])
}

/// The versions the changelog has a section for: `## [1.2.0] - date`,
/// `## 1.2.0`, `## [v1.2.0]`, `## 🎉 [1.2.0]`.
fn sections(text: &str) -> Vec<String> {
    headings(text).into_iter().map(|(version, _)| version).collect()
}

/// Each version section, newest first, with what its heading says after the
/// number: usually the day.
pub fn headings(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|line| line.strip_prefix("## "))
        .filter_map(|heading| {
            // A heading may open with an emoji the changelog's template puts there.
            let heading = heading.trim_start_matches(|c: char| !c.is_ascii_alphanumeric() && c != '[');
            let number = heading.trim_start_matches('[').split([']', ' ', '\t']).next()?;
            let rest = heading[heading.find(number)? + number.len()..].trim_start_matches(']').trim();
            let number = number.trim_start_matches('v');
            let parts: Vec<&str> = number.split(['.', '-', '+']).collect();
            let valid = parts.len() >= 3 && parts[..3].iter().all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()));
            valid.then(|| {
                (
                    number.to_string(),
                    rest.trim_start_matches(['-', '(', ' ']).trim_end_matches(')').trim().to_string(),
                )
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_section_is_found_however_its_heading_is_written() {
        let text = "# Changelog\n\n## [Unreleased]\n\n## [2.12.0] - 2026-10-13\n\n- x\n## v2.11.0\n## 2.10.0 (2026-10-07)\n";
        assert_eq!(sections(text), ["2.12.0", "2.11.0", "2.10.0"]);
    }
}
