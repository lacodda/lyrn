//! `installers`: the installers fetch this repository's releases, ask for the
//! newest one the way that never runs out, and suggest the crate it publishes.
//!
//! An installer is a shopfront too - the line the README tells a stranger to
//! paste. One copied from a sibling and left pointing at the sibling installs
//! the wrong program; one asking `api.github.com` for the newest tag works
//! until sixty anonymous questions an hour are spent on a shared address; one
//! suggesting `cargo install` of a name the repository does not publish
//! suggests a command that fails (kasl suggested `kasl` while publishing
//! `kasl-cli`).

use regex::Regex;

use super::product::{Kind, Product};
use crate::commands::doctor::{Finding, Outcome, Repo, plural};

const INSTALLERS: &[&str] = &["tools/install.sh", "tools/install.ps1", "install.sh", "install.ps1"];

pub fn check(repo: &Repo, product: &Product) -> Outcome {
    let present: Vec<&str> = INSTALLERS.iter().copied().filter(|i| repo.has_file(i)).collect();
    if present.is_empty() {
        return Outcome::Skip("no installers".to_string());
    }
    let crates: Vec<&str> = product
        .manifests
        .iter()
        .filter(|m| m.kind == Kind::Cargo)
        .filter_map(|m| m.name.as_deref())
        .collect();
    let cargo_install = Regex::new(r"cargo\s+install\s+(?:--locked\s+|--force\s+|-f\s+)*([A-Za-z0-9_-]+)").expect("a valid pattern");
    let mut findings = Vec::new();
    for installer in &present {
        let text = repo.read_text(installer).unwrap_or_default();
        match &product.github {
            Some(slug) if !text.contains(slug.as_str()) => findings.push(
                Finding::fail(format!("{installer} does not name {slug}, the repository its releases come from"))
                    .fix(format!("fetch from https://github.com/{slug}/releases")),
            ),
            _ => {}
        }
        if text.contains("api.github.com") {
            findings.push(Finding::fail(format!("{installer} asks api.github.com for the newest tag")).fix(
                "read it from the redirect of https://github.com/<owner>/<name>/releases/latest: the API answers sixty anonymous questions an hour per address",
            ));
        }
        for captures in cargo_install.captures_iter(&text) {
            let name = &captures[1];
            if !name.starts_with('-') && !crates.contains(&name) {
                findings.push(
                    Finding::fail(format!("{installer} suggests `cargo install {name}`, and no crate here is called that")).fix(match crates.first() {
                        Some(first) => format!("suggest the crate the repository publishes, such as `{first}`"),
                        None => "suggest a way of installing that exists".to_string(),
                    }),
                );
            }
        }
    }
    let slug = product.github.as_deref().map_or(String::new(), |s| format!(" from {s}"));
    Outcome::from_findings(findings, || {
        format!("{} fetch releases{slug} by the redirect", plural(present.len(), "installer", "installers"))
    })
}
