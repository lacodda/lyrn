//! `descriptions`: the product says the same thing about itself everywhere.
//!
//! The crate's page, the npm page, the documentation site's meta tags and the
//! repository's own caption each show one sentence about the product, and a
//! sentence rewritten in one of them is invisible in the rest until someone
//! compares them. The top manifest says it first; every other shopfront of
//! the product says it in the same words, give or take the full stop a
//! caption leaves off. A workspace has no top package to say it, so there
//! only the pairs that show one artefact twice are held: an npm wrapper and
//! the crate it installs.

use super::product::{Kind, Manifest, Product};
use super::same_words;
use super::site::Site;
use crate::commands::doctor::{Finding, Outcome, Repo};

pub fn check(repo: &Repo, product: &Product) -> Outcome {
    let source = product.description();
    let mut findings = Vec::new();
    let mut agree: Vec<String> = Vec::new();
    let mut compared: Vec<&str> = Vec::new();

    if let Some((source, description)) = source {
        for manifest in &product.manifests {
            if std::ptr::eq(manifest, source) || !product.presents(manifest) {
                continue;
            }
            compared.push(&manifest.path);
            compare(manifest, source, description, &mut findings, &mut agree);
        }
        if let Some(site) = Site::read(repo) {
            let config = format!("{}/astro.config.mjs", site.dir);
            match site.description.as_deref() {
                Some(theirs) if same_words(theirs, description) => agree.push(config),
                Some(theirs) => findings.push(
                    Finding::fail(format!("{config} describes the site as \"{theirs}\""))
                        .fix(format!("the site's meta tags say what {} says: \"{description}\"", source.path)),
                ),
                None => findings.push(
                    Finding::warn(format!("{config} sets no `description` lyrn can read")).fix("write it in place or as a constant: `const description = '…'`"),
                ),
            }
        }
    }

    // A wrapper and its crate are one program on two registries, whatever
    // else the repository holds.
    for wrapper in product
        .manifests
        .iter()
        .filter(|m| m.kind == Kind::Package && m.publishable && m.path.starts_with("npm/"))
    {
        if compared.contains(&wrapper.path.as_str()) {
            continue;
        }
        let Some(krate) = product.wrapped_crate(wrapper) else { continue };
        let Some(description) = krate.description.as_deref().filter(|d| !d.trim().is_empty()) else {
            continue;
        };
        compare(wrapper, krate, description, &mut findings, &mut agree);
    }

    if source.is_none() && agree.is_empty() && findings.is_empty() {
        return Outcome::Skip("a workspace: each crate describes itself, and no top package speaks for the whole".to_string());
    }
    Outcome::from_findings(findings, || match source {
        Some((source, _)) if agree.is_empty() => format!("{} says what the product is, and nothing else describes it", source.path),
        Some((source, _)) => format!("{} and {} say the same", source.path, agree.join(", ")),
        None => format!("{} say what the crates they install say", agree.join(", ")),
    })
}

fn compare(manifest: &Manifest, source: &Manifest, description: &str, findings: &mut Vec<Finding>, agree: &mut Vec<String>) {
    match manifest.description.as_deref().filter(|d| !d.trim().is_empty()) {
        Some(theirs) if same_words(theirs, description) => agree.push(manifest.path.clone()),
        Some(theirs) => {
            findings.push(Finding::fail(format!("{} says \"{theirs}\"", manifest.path)).fix(format!("say what {} says: \"{description}\"", source.path)))
        }
        None if manifest.publishable => findings.push(
            Finding::fail(format!("{} has no description, so its registry page says nothing", manifest.path))
                .fix(format!("\"description\": \"{description}\"")),
        ),
        None => {}
    }
}
