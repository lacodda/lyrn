//! `lyrn versions`: where the version is written, and where it went.
//!
//! One screen for the question every release ends with - did it all move? -
//! asked of the files and of the places the release goes: the manifests and
//! the wrapper's pin, Cargo.lock and the changelog, the tag, GitHub's latest
//! release, crates.io and npm. It says; it does not hold anything to a rule.
//! `lyrn check versions` is the gate.

use std::error::Error;
use std::path::PathBuf;

use crate::cli::PathArgs;
use crate::commands::check::changelog;
use crate::commands::check::product::{Kind, Product};
use crate::commands::doctor::Repo;
use crate::commands::standing::Standing;
use crate::registries;

pub fn run(args: PathArgs) -> Result<(), Box<dyn Error>> {
    let path = args.path.unwrap_or_else(|| PathBuf::from("."));
    if !path.is_dir() {
        return Err(format!("`{}` is not a directory - `lyrn versions` reads a repository", path.display()).into());
    }
    let repo = Repo::read(&path);
    let product = Product::read(&repo);
    let mut rows: Vec<(String, String)> = Vec::new();

    for (manifest, version) in product.versioned() {
        rows.push((version.to_string(), manifest.path.clone()));
    }
    for manifest in product.manifests.iter().filter(|m| m.kind == Kind::Package) {
        for (key, pin) in &manifest.binary_pins {
            rows.push((pin.clone(), format!("{} ({key})", manifest.path)));
        }
    }
    for lock in repo.files_named("Cargo.lock").filter(|l| !crate::commands::doctor::is_test_material(l)) {
        let Some(table) = repo.read_text(lock).and_then(|t| toml::from_str::<toml::Table>(&t).ok()) else {
            continue;
        };
        for package in table.get("package").and_then(|p| p.as_array()).into_iter().flatten() {
            if package.get("source").is_some() {
                continue;
            }
            let (Some(name), Some(version)) = (package.get("name").and_then(|n| n.as_str()), package.get("version").and_then(|v| v.as_str())) else {
                continue;
            };
            if product.owns(name) || name == product.name {
                rows.push((version.to_string(), format!("{lock} ({name})")));
            }
        }
    }
    if let Some((version, rest)) = repo.read_text("CHANGELOG.md").and_then(|text| changelog::headings(&text).into_iter().next()) {
        let when = if rest.is_empty() { String::new() } else { format!(", {rest}") };
        rows.push((version, format!("CHANGELOG.md, the newest section{when}")));
    }

    let heading = match product.version() {
        Some((_, version)) => format!("{} {version}", product.name),
        None => product.name.clone(),
    };
    println!("lyrn versions: {}, {heading}\n", repo.root.display());
    println!("Written");
    print_rows(&rows);

    let mut released: Vec<(String, String)> = Vec::new();
    if let Some(git) = &repo.git {
        match Standing::read(git).release {
            Some(release) => {
                let since = match release.since.len() {
                    0 => "nothing committed since".to_string(),
                    1 => "1 commit since".to_string(),
                    n => format!("{n} commits since"),
                };
                let commit = &release.commit[..release.commit.len().min(7)];
                released.push((release.tag.clone(), format!("tagged {} on {commit}, {since}", release.day)));
            }
            None => released.push(("-".to_string(), "no version tag behind this commit".to_string())),
        }
    }
    if let Some(slug) = &product.github {
        released.push(answer(registries::github_latest(slug), format!("GitHub's latest release of {slug}")));
    }
    let mut asked: Vec<(Kind, String)> = Vec::new();
    for manifest in &product.manifests {
        let Some(name) = manifest.name.as_deref() else { continue };
        if !manifest.publishable || !product.owns(name) || matches!(manifest.kind, Kind::Tauri) {
            continue;
        }
        if !asked.iter().any(|(k, n)| *k == manifest.kind && n == name) {
            asked.push((manifest.kind, name.to_string()));
        }
    }
    let answers: Vec<(String, String)> = std::thread::scope(|scope| {
        let handles: Vec<_> = asked
            .iter()
            .map(|(kind, name)| {
                scope.spawn(move || match kind {
                    Kind::Cargo => answer(registries::newest_crate(name), format!("crates.io: {name}")),
                    _ => answer(registries::npm_latest(name), format!("npm: {name}")),
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().expect("asking a registry does not panic")).collect()
    });
    released.extend(answers);
    println!("\nReleased");
    print_rows(&released);

    let written: Vec<&str> = rows.iter().map(|(v, _)| v.trim_start_matches('v')).collect();
    let mut distinct: Vec<&str> = written.clone();
    distinct.sort();
    distinct.dedup();
    println!();
    match distinct.as_slice() {
        [] => println!("No file writes a version."),
        [one] => println!("Every file says {one}."),
        many => println!("The files disagree: {} - `lyrn check versions` says which to change.", many.join(", ")),
    }
    Ok(())
}

/// A registry's answer as a row: the version, or a dash and why not.
fn answer(result: Result<Option<String>, String>, place: String) -> (String, String) {
    match result {
        Ok(Some(version)) => (version, place),
        Ok(None) => ("-".to_string(), format!("{place}, nothing there yet")),
        Err(reason) => ("?".to_string(), format!("{place}, could not ask - {reason}")),
    }
}

fn print_rows(rows: &[(String, String)]) {
    let width = rows.iter().map(|(v, _)| v.chars().count()).max().unwrap_or(1) + 2;
    for (version, place) in rows {
        println!("  {version:<width$}{place}");
    }
}
