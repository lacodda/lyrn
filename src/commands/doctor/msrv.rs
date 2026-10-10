//! `msrv`: `rust-version` is declared, and a CI job builds on it.
//!
//! The number is a promise to everyone running `cargo install`, and Cargo
//! believes it. Nothing checks it on its own: the first time the code uses a
//! newer API the promise is broken and nobody notices, so the standard pairs
//! the number with a job that reads it from the manifest and builds on it.

use super::workflows::Job;
use super::{Finding, Outcome, Repo};

/// The manifests a repository's Rust lives in: the top, and a desktop app's
/// shell.
const MANIFESTS: &[&str] = &["Cargo.toml", "src-tauri/Cargo.toml"];

pub fn check(repo: &Repo) -> Outcome {
    let manifests: Vec<&str> = MANIFESTS.iter().copied().filter(|m| repo.has_file(m) || repo.root.join(m).is_file()).collect();
    if manifests.is_empty() {
        return Outcome::Skip("not a Rust project".to_string());
    }

    let mut findings = Vec::new();
    let mut declared: Vec<String> = Vec::new();
    for manifest in &manifests {
        match declared_version(repo, manifest) {
            Ok(Some(version)) => declared.push(version),
            Ok(None) => {
                // A virtual workspace has no package to declare it for; its
                // members inherit `[workspace.package]`, read above.
                findings.push(
                    Finding::fail(format!("{manifest} declares no rust-version"))
                        .fix("measure the oldest toolchain that builds it and write it as `rust-version`"),
                )
            }
            Err(reason) => findings.push(Finding::fail(format!("{manifest} does not parse: {reason}"))),
        }
    }
    let Some(declared) = declared.into_iter().max_by(|a, b| compare(a, b)) else {
        return Outcome::Found(findings);
    };

    let jobs: Vec<(String, Job)> = repo
        .workflows
        .iter()
        .flat_map(|w| w.jobs().into_iter().map(move |j| (w.name().to_string(), j)))
        .filter(|(_, job)| job.id.eq_ignore_ascii_case("msrv") || job.name.is_some_and(|n| n.eq_ignore_ascii_case("msrv")))
        .collect();
    let Some((file, job)) = jobs.first() else {
        findings.push(
            Finding::fail(format!("rust-version {declared} is declared, and no CI job builds on it"))
                .fix("add an `msrv` job that reads it with `cargo metadata` and builds with that toolchain - the one `lyrn new` writes"),
        );
        return Outcome::Found(findings);
    };

    match toolchain_of(job) {
        Toolchain::Literal(version) if compare(&version, &declared).is_ne() => findings.push(
            Finding::fail(format!("the msrv job in {file} builds on {version}, and the manifest says {declared}"))
                .fix("read the number from the manifest instead of writing it twice"),
        ),
        Toolchain::Literal(version) => findings.push(
            Finding::warn(format!(
                "the msrv job in {file} writes {version} by hand: it agrees today, and stops agreeing at the next bump"
            ))
            .fix("read it with `cargo metadata --no-deps --format-version 1 | jq -r '.packages[0].rust_version'`"),
        ),
        Toolchain::Channel(channel) => findings.push(
            Finding::fail(format!("the msrv job in {file} builds on `{channel}`, not on rust-version {declared}"))
                .fix("give the toolchain the number read from the manifest"),
        ),
        Toolchain::FromManifest => {}
    }

    Outcome::from_findings(findings, || format!("rust-version {declared}, held by the `{}` job in {file}", job.id))
}

/// `rust-version` of a manifest, following `rust-version.workspace = true` to
/// the workspace at the top.
fn declared_version(repo: &Repo, manifest: &str) -> Result<Option<String>, String> {
    let text = repo.read_text(manifest).ok_or_else(|| "it cannot be read".to_string())?;
    let table: toml::Table = toml::from_str(&text).map_err(|e| e.to_string())?;
    let workspace_version = || -> Option<String> {
        let top: toml::Table = if manifest == "Cargo.toml" {
            table.clone()
        } else {
            toml::from_str(&repo.read_text("Cargo.toml")?).ok()?
        };
        top.get("workspace")?.get("package")?.get("rust-version")?.as_str().map(str::to_string)
    };
    match table.get("package").and_then(|p| p.get("rust-version")) {
        Some(toml::Value::String(version)) => Ok(Some(version.clone())),
        Some(toml::Value::Table(t)) if t.get("workspace").and_then(|w| w.as_bool()) == Some(true) => Ok(workspace_version()),
        Some(_) => Err("rust-version is neither a version nor `workspace = true`".to_string()),
        None if table.get("package").is_none() => Ok(workspace_version()),
        None => Ok(None),
    }
}

enum Toolchain {
    /// A version written into the workflow.
    Literal(String),
    /// A channel: `stable`, `nightly`.
    Channel(String),
    /// Read at run time - from the manifest, the job says.
    FromManifest,
}

fn toolchain_of(job: &Job) -> Toolchain {
    let reads_manifest = job
        .steps
        .iter()
        .filter_map(|s| s.run)
        .any(|r| r.contains("rust_version") || r.contains("rust-version"));
    for step in &job.steps {
        let Some(uses) = step.uses else { continue };
        let Some((action, reference)) = uses.split_once('@') else { continue };
        if !action.ends_with("rust-toolchain") {
            continue;
        }
        let chosen = step.input("toolchain").unwrap_or_else(|| reference.to_string());
        if chosen.contains("${{") {
            return if reads_manifest {
                Toolchain::FromManifest
            } else {
                Toolchain::Channel(chosen)
            };
        }
        if chosen.starts_with(|c: char| c.is_ascii_digit()) {
            return Toolchain::Literal(chosen);
        }
        if chosen != "master" && chosen != "main" {
            return Toolchain::Channel(chosen);
        }
    }
    // `rustup toolchain install` in a script, or anything else: trust a job
    // that reads the number, and name what it builds on otherwise.
    if reads_manifest {
        Toolchain::FromManifest
    } else {
        Toolchain::Channel("whatever the runner has".to_string())
    }
}

/// Versions by their numbers: `1.9` is older than `1.85`.
fn compare(a: &str, b: &str) -> std::cmp::Ordering {
    let parts = |v: &str| -> Vec<u64> { v.split('.').take(2).map(|p| p.parse().unwrap_or(0)).collect() };
    parts(a).cmp(&parts(b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::doctor::workflows;

    fn job_of(yaml: &str) -> Toolchain {
        let workflow = workflows::parse("ci.yml", yaml.to_string()).unwrap();
        toolchain_of(&workflow.jobs()[0])
    }

    #[test]
    fn the_job_the_templates_write_reads_the_manifest() {
        let yaml = "jobs:\n  msrv:\n    steps:\n      - id: msrv\n        run: echo \"version=$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[0].rust_version')\" >> \"$GITHUB_OUTPUT\"\n      - uses: dtolnay/rust-toolchain@master\n        with:\n          toolchain: ${{ steps.msrv.outputs.version }}\n";
        assert!(matches!(job_of(yaml), Toolchain::FromManifest));
    }

    #[test]
    fn a_number_or_a_channel_in_the_workflow_is_named() {
        assert!(matches!(job_of("jobs:\n  msrv:\n    steps:\n      - uses: dtolnay/rust-toolchain@1.85\n"), Toolchain::Literal(v) if v == "1.85"));
        assert!(matches!(
            job_of("jobs:\n  msrv:\n    steps:\n      - uses: dtolnay/rust-toolchain@master\n        with:\n          toolchain: 1.80.0\n"),
            Toolchain::Literal(v) if v == "1.80.0"
        ));
        assert!(matches!(job_of("jobs:\n  msrv:\n    steps:\n      - uses: dtolnay/rust-toolchain@stable\n"), Toolchain::Channel(c) if c == "stable"));
    }

    #[test]
    fn versions_compare_by_number() {
        assert!(compare("1.9", "1.85").is_lt());
        assert!(compare("1.88", "1.88.0").is_eq());
    }
}
