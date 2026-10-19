//! `lyrn deps`: when the stack was last brought up to date, and which
//! dependencies are behind.
//!
//! The line brings its whole stack up to date at the start of a version, at
//! most once a week, in one `chore(deps)` commit - majors included, with the
//! code fixed in the same commit. Whether the week has passed is read from the
//! history, by the fact. What to update is read from the lockfiles against the
//! registries: the version a build actually uses, beside the newest one there
//! is.

use std::error::Error;
use std::path::PathBuf;
use std::process::Command as Process;

use chrono::Utc;
use yaml_rust2::{Yaml, YamlLoader};

use crate::cli::PathArgs;
use crate::commands::doctor::stack::{Stack, Standard};
use crate::commands::doctor::{Repo, is_test_material, stack_line};
use crate::commands::standing::stack_text;
use crate::registries::{self, breaking, version_cmp};

/// A direct dependency, as the lockfile has it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Locked {
    name: String,
    version: String,
}

/// The direct dependencies of one manifest, and the registry they come from.
struct Group {
    manifest: String,
    registry: Registry,
    locked: Vec<Locked>,
}

/// A registry's answer about one package: its newest version, or why there
/// is none.
type Answer = ((Registry, String), Result<Option<String>, String>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Registry {
    Crates,
    Npm,
}

pub fn run(args: PathArgs) -> Result<(), Box<dyn Error>> {
    let path = args.path.unwrap_or_else(|| PathBuf::from("."));
    if !path.is_dir() {
        return Err(format!("`{}` is not a directory - `lyrn deps` reads a repository", path.display()).into());
    }
    let repo = Repo::read(&path);
    let now = Utc::now();
    let stack = Stack::read(&repo, now);
    let standard = Standard::read();
    println!("lyrn deps: {}\n", repo.root.display());

    if let Some(updated) = &stack.updated {
        let (text, due) = stack_text(updated, now.date_naive());
        println!("stack     {text}");
        if due {
            println!("          the next stage starts with one `chore(deps)` commit: majors too, the code fixed in it");
        }
        if let Some(tag) = &updated.skipped_by {
            println!("          {tag} began on a stack a week old or more and shipped without updating it");
        }
    }
    if let Some(line) = stack_line(&stack, &standard) {
        // The update has a line of its own above.
        let cells: Vec<&str> = line.split(" · ").filter(|c| !c.starts_with("updated")).collect();
        if !cells.is_empty() {
            println!("on        {}", cells.join(" · "));
        }
    }
    if repo.has_file("Cargo.toml")
        && let Some(line) = rustup_check()
    {
        println!("rust      {line}");
    }

    let groups: Vec<Group> = cargo_groups(&repo)
        .into_iter()
        .chain(npm_groups(&repo))
        .filter(|g| !g.locked.is_empty())
        .collect();
    if groups.is_empty() {
        println!("\nNo lockfile to read dependencies from.");
        return Ok(());
    }
    // Every name once, asked side by side: the registries have nothing to
    // wait for between questions.
    let mut names: Vec<(Registry, String)> = groups.iter().flat_map(|g| g.locked.iter().map(|l| (g.registry, l.name.clone()))).collect();
    names.sort_by(|a, b| (a.0 as u8, &a.1).cmp(&(b.0 as u8, &b.1)));
    names.dedup();
    let newest: Vec<Answer> = std::thread::scope(|scope| {
        names
            .chunks(8)
            .flat_map(|chunk| {
                let handles: Vec<_> = chunk
                    .iter()
                    .map(|(registry, name)| {
                        scope.spawn(move || {
                            let answer = match registry {
                                Registry::Crates => registries::newest_crate(name),
                                Registry::Npm => registries::npm_latest(name),
                            };
                            ((*registry, name.clone()), answer)
                        })
                    })
                    .collect();
                handles
                    .into_iter()
                    .map(|h| h.join().expect("asking a registry does not panic"))
                    .collect::<Vec<_>>()
            })
            .collect()
    });
    let lookup = |registry: Registry, name: &str| newest.iter().find(|((r, n), _)| *r == registry && n == name).map(|(_, a)| a);

    let (mut behind, mut majors, mut unknown) = (0, 0, 0);
    for group in &groups {
        let registry = match group.registry {
            Registry::Crates => "crates.io",
            Registry::Npm => "npm",
        };
        println!("\n{} - {registry}", group.manifest);
        let mut current = 0;
        let width = group.locked.iter().map(|l| l.name.len()).max().unwrap_or(4) + 2;
        let version_width = group.locked.iter().map(|l| l.version.len()).max().unwrap_or(5) + 2;
        for locked in &group.locked {
            match lookup(group.registry, &locked.name) {
                Some(Ok(Some(newest))) if version_cmp(&locked.version, newest).is_lt() => {
                    behind += 1;
                    let major = breaking(&locked.version, newest);
                    if major {
                        majors += 1;
                    }
                    let note = if major { "  a breaking version" } else { "" };
                    println!("  {:<width$}{:<version_width$}→ {newest}{note}", locked.name, locked.version);
                }
                Some(Ok(_)) => current += 1,
                Some(Err(reason)) => {
                    unknown += 1;
                    println!(
                        "  {:<width$}{:<version_width$}? could not ask {registry} - {reason}",
                        locked.name, locked.version
                    );
                }
                None => {}
            }
        }
        if current > 0 {
            println!("  ({current} up to date)");
        }
    }
    println!();
    let mut summary = match behind {
        0 => "Every dependency is on its newest version".to_string(),
        1 => "1 dependency behind".to_string(),
        n => format!("{n} dependencies behind"),
    };
    if majors > 0 {
        summary.push_str(&format!(", {majors} of them across a breaking version"));
    }
    if unknown > 0 {
        summary.push_str(&format!("; {unknown} could not be asked about"));
    }
    println!("{summary}.");
    Ok(())
}

/// What `rustup check` says about the stable toolchain, when rustup is here.
fn rustup_check() -> Option<String> {
    let out = Process::new("rustup").arg("check").output().ok()?;
    parse_rustup_check(&String::from_utf8_lossy(&out.stdout))
}

/// `stable-<host> - up to date: 1.99.1 (…)`, or `- update available: 1.98.0
/// (…) -> 1.99.1 (…)`; rustup has written both with and without capitals.
fn parse_rustup_check(text: &str) -> Option<String> {
    let line = text.lines().find(|l| l.starts_with("stable-"))?;
    let (_, state) = line.split_once(" - ")?;
    let (status, versions) = state.split_once(':')?;
    let version = |s: &str| s.split_whitespace().next().unwrap_or_default().to_string();
    if status.trim().eq_ignore_ascii_case("update available") {
        let (from, to) = versions.split_once("->")?;
        Some(format!("stable {} here, {} is out - `rustup update stable`", version(from), version(to)))
    } else {
        Some(format!("stable {}, the newest", version(versions)))
    }
}

/// The direct dependencies of every package the repository builds, at the
/// versions Cargo.lock resolves them to.
fn cargo_groups(repo: &Repo) -> Vec<Group> {
    let mut groups = Vec::new();
    for lock in repo.files_named("Cargo.lock").filter(|l| !is_test_material(l)) {
        let dir = lock.strip_suffix("Cargo.lock").unwrap_or_default();
        let Some(table) = repo.read_text(lock).and_then(|t| toml::from_str::<toml::Table>(&t).ok()) else {
            continue;
        };
        let packages: Vec<&toml::Table> = table
            .get("package")
            .and_then(|p| p.as_array())
            .into_iter()
            .flatten()
            .filter_map(|p| p.as_table())
            .collect();
        let field = |p: &toml::Table, key: &str| p.get(key).and_then(|v| v.as_str()).map(str::to_string);
        let version_of = |name: &str, pinned: Option<&str>| -> Option<String> {
            if let Some(pinned) = pinned {
                return Some(pinned.to_string());
            }
            let found: Vec<String> = packages
                .iter()
                .filter(|p| field(p, "name").as_deref() == Some(name))
                .filter_map(|p| field(p, "version"))
                .collect();
            (found.len() == 1).then(|| found[0].clone())
        };
        // The packages built here have no source; each lists what it uses.
        for package in packages.iter().filter(|p| p.get("source").is_none()) {
            let Some(name) = field(package, "name") else { continue };
            let manifest = repo
                .files_named("Cargo.toml")
                .filter(|m| m.starts_with(dir) && !is_test_material(m))
                .find(|m| {
                    repo.read_text(m)
                        .and_then(|t| toml::from_str::<toml::Table>(&t).ok())
                        .and_then(|t| t.get("package")?.get("name")?.as_str().map(|n| n == name))
                        .unwrap_or(false)
                })
                .map(str::to_string)
                .unwrap_or_else(|| format!("{dir}Cargo.toml ({name})"));
            let mut locked = Vec::new();
            for dependency in package
                .get("dependencies")
                .and_then(|d| d.as_array())
                .into_iter()
                .flatten()
                .filter_map(|d| d.as_str())
            {
                let mut words = dependency.split_whitespace();
                let Some(dep) = words.next() else { continue };
                // A sibling built here is not on crates.io.
                if packages.iter().any(|p| p.get("source").is_none() && field(p, "name").as_deref() == Some(dep)) {
                    continue;
                }
                if let Some(version) = version_of(dep, words.next()) {
                    locked.push(Locked {
                        name: dep.to_string(),
                        version,
                    });
                }
            }
            locked.sort_by(|a, b| a.name.cmp(&b.name));
            groups.push(Group {
                manifest,
                registry: Registry::Crates,
                locked,
            });
        }
    }
    groups
}

/// The direct dependencies of every package.json beside a pnpm lockfile, at
/// the versions the lockfile resolves them to.
fn npm_groups(repo: &Repo) -> Vec<Group> {
    let mut groups = Vec::new();
    for lock in repo.files_named("pnpm-lock.yaml").filter(|l| !is_test_material(l)) {
        let dir = lock.strip_suffix("pnpm-lock.yaml").unwrap_or_default();
        let Some(doc) = repo
            .read_text(lock)
            .and_then(|t| YamlLoader::load_from_str(&t).ok())
            .and_then(|mut d| (!d.is_empty()).then(|| d.swap_remove(0)))
        else {
            continue;
        };
        let Some(importers) = doc["importers"].as_hash() else { continue };
        for (importer, sections) in importers {
            let Some(importer) = importer.as_str() else { continue };
            let manifest = if importer == "." {
                format!("{dir}package.json")
            } else {
                format!("{dir}{importer}/package.json")
            };
            let mut locked = Vec::new();
            for section in ["dependencies", "devDependencies", "optionalDependencies"] {
                let Some(entries) = sections[section].as_hash() else { continue };
                for (name, entry) in entries {
                    let (Some(name), Some(version)) = (name.as_str(), entry["version"].as_str()) else {
                        continue;
                    };
                    // A link or a workspace sibling is not on the registry.
                    if version.starts_with("link:")
                        || version.starts_with("file:")
                        || matches!(entry["specifier"], Yaml::String(ref s) if s.starts_with("workspace:"))
                    {
                        continue;
                    }
                    let version = version.split('(').next().unwrap_or(version);
                    locked.push(Locked {
                        name: name.to_string(),
                        version: version.to_string(),
                    });
                }
            }
            locked.sort_by(|a, b| a.name.cmp(&b.name));
            groups.push(Group {
                manifest,
                registry: Registry::Npm,
                locked,
            });
        }
    }
    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_with(files: &[(&str, &str)]) -> (tempfile::TempDir, Repo) {
        let dir = tempfile::tempdir().unwrap();
        for (path, text) in files {
            let file = dir.path().join(path);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, text).unwrap();
        }
        let repo = Repo::read(dir.path());
        (dir, repo)
    }

    #[test]
    fn rustup_says_whether_stable_is_the_newest() {
        let current = "stable-x86_64-pc-windows-msvc - up to date: 1.99.1 (58cf8f9c3 2026-10-13)
rustup - up to date : 1.29.1
";
        assert_eq!(parse_rustup_check(current).as_deref(), Some("stable 1.99.1, the newest"));
        let behind = "stable-x86_64-unknown-linux-gnu - Update available : 1.98.0 (aaa 2026-09-01) -> 1.99.1 (bbb 2026-10-13)
";
        assert_eq!(
            parse_rustup_check(behind).as_deref(),
            Some("stable 1.98.0 here, 1.99.1 is out - `rustup update stable`")
        );
    }

    #[test]
    fn a_crate_s_dependencies_are_the_versions_its_lock_resolves() {
        let lock = "version = 4\n\n[[package]]\nname = \"demo\"\nversion = \"1.0.0\"\ndependencies = [\n \"demo-core\",\n \"serde\",\n \"toml 1.1.8\",\n]\n\n[[package]]\nname = \"demo-core\"\nversion = \"1.0.0\"\n\n[[package]]\nname = \"serde\"\nversion = \"1.0.229\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n\n[[package]]\nname = \"toml\"\nversion = \"0.9.0\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n\n[[package]]\nname = \"toml\"\nversion = \"1.1.8\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n";
        let (_dir, repo) = repo_with(&[("Cargo.lock", lock), ("Cargo.toml", "[package]\nname = \"demo\"\n")]);
        let groups = cargo_groups(&repo);
        let demo = groups.iter().find(|g| g.manifest == "Cargo.toml").expect("the top package is read");
        assert_eq!(
            demo.locked,
            [
                Locked {
                    name: "serde".into(),
                    version: "1.0.229".into()
                },
                Locked {
                    name: "toml".into(),
                    version: "1.1.8".into()
                }
            ]
        );
    }

    #[test]
    fn a_package_s_dependencies_are_the_versions_pnpm_resolved() {
        let lock = "lockfileVersion: '9.0'\n\nimporters:\n\n  .:\n    dependencies:\n      astro:\n        specifier: ^7.3.8\n        version: 7.3.8(@types/node@24.19.1)\n      shared:\n        specifier: workspace:*\n        version: link:../shared\n    devDependencies:\n      '@types/node':\n        specifier: ^24\n        version: 24.19.1\n";
        let (_dir, repo) = repo_with(&[("docs/pnpm-lock.yaml", lock), ("docs/package.json", "{}")]);
        let groups = npm_groups(&repo);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].manifest, "docs/package.json");
        let names: Vec<(&str, &str)> = groups[0].locked.iter().map(|l| (l.name.as_str(), l.version.as_str())).collect();
        assert_eq!(names, [("@types/node", "24.19.1"), ("astro", "7.3.8")]);
    }
}
