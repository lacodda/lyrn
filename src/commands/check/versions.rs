//! `versions`: every manifest of the product says one version.
//!
//! Each registry and installer reads the number from its own file, so a file
//! left at the old number is a shopfront advertising a release that is not
//! the one being shipped - or, for the pin an npm wrapper downloads by, a
//! package that installs the previous binary under the new version. Cargo.lock
//! records the workspace's own packages too, and a lock that still says the
//! old number is refused by every `--locked` build.

use super::product::{Kind, Product};
use crate::commands::doctor::{Finding, Outcome, Repo, plural};

pub fn check(repo: &Repo, product: &Product) -> Outcome {
    let Some((source, version)) = product.version() else {
        return Outcome::Skip("no manifest carries a version".to_string());
    };
    let mut findings = Vec::new();
    let mut places: Vec<String> = Vec::new();
    for (manifest, written) in product.versioned() {
        if written == version {
            places.push(manifest.path.clone());
        } else {
            findings.push(
                Finding::fail(format!("{} says {written}, the product is {version} ({})", manifest.path, source.path))
                    .fix(format!("write {version} into {}", manifest.path)),
            );
        }
    }

    let tag = format!("v{version}");
    for manifest in product.manifests.iter().filter(|m| m.kind == Kind::Package) {
        for (key, pinned) in &manifest.binary_pins {
            if *pinned == tag {
                places.push(format!("{} ({key})", manifest.path));
            } else {
                findings.push(
                    Finding::fail(format!("{} pins the binary at {pinned} (`{key}`), the product is {version}", manifest.path))
                        .fix(format!("set `{key}` to {tag}: the wrapper downloads the release it names")),
                );
            }
        }
    }

    findings.extend(lockfiles(repo, product, &mut places));
    findings.extend(release_toml(repo, product));

    Outcome::from_findings(findings, || {
        format!("{} {version}: {}", plural(places.len(), "place says", "places say"), places.join(", "))
    })
}

/// Each tracked Cargo.lock against the packages of its workspace.
fn lockfiles(repo: &Repo, product: &Product, places: &mut Vec<String>) -> Vec<Finding> {
    let mut findings = Vec::new();
    for lock in repo.files_named("Cargo.lock") {
        if crate::commands::doctor::is_test_material(lock) {
            continue;
        }
        let dir = lock.strip_suffix("Cargo.lock").unwrap_or_default();
        let Some(text) = repo.read_text(lock) else { continue };
        let Ok(table) = toml::from_str::<toml::Table>(&text) else {
            findings.push(Finding::fail(format!("{lock} does not parse")));
            continue;
        };
        // The packages built here: those without a source.
        let locked: Vec<(String, String)> = table
            .get("package")
            .and_then(|p| p.as_array())
            .into_iter()
            .flatten()
            .filter(|p| p.get("source").is_none())
            .filter_map(|p| Some((p.get("name")?.as_str()?.to_string(), p.get("version")?.as_str()?.to_string())))
            .collect();
        let workspace = product
            .manifests
            .iter()
            .find(|m| m.kind == Kind::Cargo && m.path == format!("{dir}Cargo.toml"))
            .and_then(|m| m.workspace_version.clone());
        let mut matched = 0;
        for manifest in product.manifests.iter().filter(|m| m.kind == Kind::Cargo && m.path.starts_with(dir)) {
            let Some(name) = &manifest.name else { continue };
            let wanted = if manifest.inherits { workspace.clone() } else { manifest.version.clone() };
            let Some(wanted) = wanted else { continue };
            match locked.iter().find(|(n, _)| n == name) {
                Some((_, recorded)) if *recorded == wanted => matched += 1,
                Some((_, recorded)) => findings.push(
                    Finding::fail(format!("{lock} still records {name} {recorded}, {} says {wanted}", manifest.path))
                        .fix("build once, and commit the lockfile with the manifest: a `--locked` build refuses a lock that disagrees"),
                ),
                None => {}
            }
        }
        if matched > 0 {
            places.push(lock.to_string());
        }
    }
    findings
}

/// The release engine rewrites the files `release.toml` lists, and only
/// those: a manifest of the product missing from the list stays at the old
/// number on release day.
fn release_toml(repo: &Repo, product: &Product) -> Vec<Finding> {
    if !repo.has_file("release.toml") {
        return Vec::new();
    }
    let Some(table) = repo.read_text("release.toml").and_then(|t| toml::from_str::<toml::Table>(&t).ok()) else {
        return vec![Finding::fail("release.toml does not parse")];
    };
    let listed: Vec<&str> = table
        .get("project")
        .and_then(|p| p.get("manifests"))
        .and_then(|m| m.as_array())
        .into_iter()
        .flatten()
        .filter_map(|m| m.as_str())
        .collect();
    product
        .versioned()
        .into_iter()
        .filter(|(m, _)| !m.inherits && !listed.contains(&m.path.as_str()))
        .map(|(m, _)| {
            Finding::fail(format!(
                "release.toml does not list {}, so the release would leave it at the old number",
                m.path
            ))
            .fix(format!("add \"{}\" to `project.manifests` in release.toml", m.path))
        })
        .collect()
}
