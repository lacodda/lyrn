//! `audit`: every lockfile in the repository is read by an audit job.
//!
//! The standard turns Dependabot's pull requests off and keeps its other half
//! as an audit workflow. That trade only holds if the audit sees everything
//! Dependabot would have: a documentation site, a frontend folder, a desktop
//! app's Rust shell each bring a lockfile of their own, and a job that audits
//! the top of the repository reads none of them.

use std::collections::BTreeSet;

use super::{Finding, Outcome, Repair, Repo, is_test_material, plural};
use crate::workflows::{Job, Step, directories, join_dir, normalize_dir};

/// A lockfile kind, and how its audit is recognised.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Ecosystem {
    Cargo,
    Pnpm,
    Npm,
}

impl Ecosystem {
    fn of(file: &str) -> Option<Ecosystem> {
        match file.rsplit('/').next()? {
            "Cargo.lock" => Some(Ecosystem::Cargo),
            "pnpm-lock.yaml" => Some(Ecosystem::Pnpm),
            "package-lock.json" => Some(Ecosystem::Npm),
            _ => None,
        }
    }

    fn command(self) -> &'static str {
        match self {
            Ecosystem::Cargo => "cargo deny check",
            Ecosystem::Pnpm => "pnpm audit",
            Ecosystem::Npm => "npm audit",
        }
    }
}

/// The workflow the docs form writes for its site: what a site under docs/
/// that nothing audits is given by `--fix`.
const DOCS_AUDIT: &str = ".github/workflows/docs-audit.yml";

pub fn check(repo: &Repo) -> Outcome {
    let lockfiles: Vec<(&str, Ecosystem)> = repo
        .files
        .iter()
        // A lockfile in a test's material locks nothing that ships.
        .filter(|f| !is_test_material(f))
        .filter_map(|f| Ecosystem::of(f).map(|e| (f.as_str(), e)))
        .collect();
    if lockfiles.is_empty() {
        return Outcome::Skip("no lockfiles".to_string());
    }

    let covered = covered(repo);
    let mut findings = Vec::new();
    for (lockfile, ecosystem) in &lockfiles {
        let dir = match lockfile.rsplit_once('/') {
            Some((dir, _)) => dir.to_string(),
            None => ".".to_string(),
        };
        if covered.contains(&(*ecosystem, dir.clone())) {
            continue;
        }
        let fix = if dir == "." {
            format!("an audit job running `{}` - `lyrn adopt` adds the standard's audit.yml", ecosystem.command())
        } else if dir == "docs" && *ecosystem == Ecosystem::Pnpm {
            "a job running `pnpm audit` with `working-directory: docs` - the docs form's docs-audit.yml is one".to_string()
        } else {
            format!("a job running `{}` with `working-directory: {dir}`", ecosystem.command())
        };
        let mut finding = Finding::fail(format!("{lockfile}: no workflow audits it")).fix(fix);
        if *lockfile == "docs/pnpm-lock.yaml" && !repo.root.join(DOCS_AUDIT).exists() {
            finding = finding.repair(Repair::new(
                DOCS_AUDIT,
                include_str!("../../templates/docs/github/docs-audit.yml.tmpl"),
                "audits the documentation site's packages, as the docs form does",
            ));
        }
        findings.push(finding);
    }

    Outcome::from_findings(findings, || format!("{} audited", plural(lockfiles.len(), "lockfile", "lockfiles")))
}

/// Every (ecosystem, directory) some workflow audits.
fn covered(repo: &Repo) -> BTreeSet<(Ecosystem, String)> {
    let mut covered = BTreeSet::new();
    for workflow in &repo.workflows {
        for job in workflow.jobs() {
            for step in &job.steps {
                audited_by(&job, step, &mut covered);
            }
        }
    }
    covered
}

fn audited_by(job: &Job, step: &Step, covered: &mut BTreeSet<(Ecosystem, String)>) {
    if let Some(uses) = step.uses {
        let action = uses.split('@').next().unwrap_or(uses);
        // cargo-deny-action runs every check unless told which, and only the
        // advisories are an audit.
        if action.eq_ignore_ascii_case("EmbarkStudios/cargo-deny-action") {
            let command = step.input("command").unwrap_or_default();
            if checks_advisories(command.split_whitespace().skip_while(|w| *w != "check").skip(1)) {
                let manifest = step.input("manifest-path").unwrap_or_else(|| "Cargo.toml".to_string());
                if let Some(dir) = manifest_dir(".", &manifest) {
                    covered.insert((Ecosystem::Cargo, dir));
                }
            }
        }
        if action.eq_ignore_ascii_case("rustsec/audit-check") || action.eq_ignore_ascii_case("actions-rust-lang/audit") {
            let dir = step
                .input("working-directory")
                .or_else(|| step.input("workingDirectory"))
                .unwrap_or_else(|| ".".to_string());
            if let Some(dir) = normalize_dir(&dir) {
                covered.insert((Ecosystem::Cargo, dir));
            }
        }
    }

    let Some(script) = step.run else { return };
    // A matrix directory is each directory the matrix lists: one job audits
    // them all, and each one is covered.
    let Some(starts) = directories(step.run_directory(job).unwrap_or("."), job) else {
        return;
    };
    for start in starts {
        audited_from(&start, script, covered);
    }
}

/// What one script audits, started in `start`.
fn audited_from(start: &str, script: &str, covered: &mut BTreeSet<(Ecosystem, String)>) {
    let mut here = start.to_string();
    for command in script.lines().flat_map(|l| l.split("&&")).flat_map(|c| c.split(';')) {
        let words: Vec<&str> = command.split_whitespace().collect();
        match words.as_slice() {
            ["cd", dir, ..] => {
                if let Some(next) = join_dir(&here, dir) {
                    here = next;
                }
            }
            ["cargo", rest @ ..] if rest.contains(&"deny") => {
                let after_check = rest.iter().skip_while(|w| **w != "check").skip(1).copied();
                if rest.contains(&"check") && checks_advisories(after_check) {
                    let dir = flag(rest, &["--manifest-path"]).map_or(Some(here.clone()), |m| manifest_dir(&here, m));
                    if let Some(dir) = dir {
                        covered.insert((Ecosystem::Cargo, dir));
                    }
                }
            }
            ["cargo", "audit", rest @ ..] => {
                let dir = flag(rest, &["--file", "-f"]).map_or(Some(here.clone()), |lock| manifest_dir(&here, lock));
                if let Some(dir) = dir {
                    covered.insert((Ecosystem::Cargo, dir));
                }
            }
            ["pnpm", rest @ ..] if rest.contains(&"audit") => {
                let dir = flag(rest, &["--dir", "-C"]).map_or(Some(here.clone()), |d| join_dir(&here, d));
                if let Some(dir) = dir {
                    covered.insert((Ecosystem::Pnpm, dir));
                }
            }
            ["npm", rest @ ..] if rest.contains(&"audit") => {
                let dir = flag(rest, &["--prefix"]).map_or(Some(here.clone()), |d| join_dir(&here, d));
                if let Some(dir) = dir {
                    covered.insert((Ecosystem::Npm, dir));
                }
            }
            _ => {}
        }
    }
}

/// Whether a `cargo deny check` with these checks named reads advisories:
/// with none named it runs them all.
fn checks_advisories<'a>(named: impl Iterator<Item = &'a str>) -> bool {
    const CHECKS: &[&str] = &["advisories", "bans", "licenses", "sources"];
    let named: Vec<&str> = named.filter(|w| CHECKS.contains(w)).collect();
    named.is_empty() || named.contains(&"advisories")
}

/// The value after one of `names`, as `--dir docs` or `--dir=docs`.
fn flag<'a>(words: &[&'a str], names: &[&str]) -> Option<&'a str> {
    for (i, word) in words.iter().enumerate() {
        for name in names {
            if word == name {
                return words.get(i + 1).copied();
            }
            if let Some(value) = word.strip_prefix(&format!("{name}=")) {
                return Some(value);
            }
        }
    }
    None
}

/// The directory of a manifest or lockfile path given from `base`.
fn manifest_dir(base: &str, path: &str) -> Option<String> {
    let path = path.trim_matches(|c| c == '"' || c == '\'');
    let dir = match path.rsplit_once('/') {
        Some((dir, _)) => dir,
        None => ".",
    };
    join_dir(base, dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflows;

    fn coverage(yaml: &str) -> Vec<(Ecosystem, String)> {
        let workflow = workflows::parse("audit.yml", yaml.to_string()).unwrap();
        let mut covered = BTreeSet::new();
        for job in workflow.jobs() {
            for step in &job.steps {
                audited_by(&job, step, &mut covered);
            }
        }
        covered.into_iter().collect()
    }

    #[test]
    fn the_standard_audit_covers_the_top() {
        let yaml = "jobs:\n  crates:\n    steps:\n      - uses: EmbarkStudios/cargo-deny-action@v2\n  packages:\n    steps:\n      - run: pnpm audit --prod\n";
        assert_eq!(coverage(yaml), vec![(Ecosystem::Cargo, ".".into()), (Ecosystem::Pnpm, ".".into())]);
    }

    #[test]
    fn a_job_audits_where_its_defaults_and_its_flags_say() {
        let yaml = "jobs:\n  site:\n    defaults:\n      run:\n        working-directory: docs\n    steps:\n      - run: pnpm install --frozen-lockfile\n      - run: pnpm audit\n  shell:\n    steps:\n      - uses: EmbarkStudios/cargo-deny-action@v2\n        with:\n          manifest-path: src-tauri/Cargo.toml\n  frontend:\n    steps:\n      - run: |\n          cd frontend\n          pnpm audit --prod\n      - run: pnpm --dir web audit\n";
        assert_eq!(
            coverage(yaml),
            vec![
                (Ecosystem::Cargo, "src-tauri".into()),
                (Ecosystem::Pnpm, "docs".into()),
                (Ecosystem::Pnpm, "frontend".into()),
                (Ecosystem::Pnpm, "web".into()),
            ]
        );
    }

    /// One job auditing every project its matrix lists covers each of them.
    #[test]
    fn a_matrix_job_audits_every_directory_it_lists() {
        let yaml = "jobs:
  packages:
    strategy:
      matrix:
        project: [web, docs]
    defaults:
      run:
        working-directory: ${{ matrix.project }}
    steps:
      - run: pnpm install --frozen-lockfile
      - run: pnpm audit
";
        assert_eq!(coverage(yaml), vec![(Ecosystem::Pnpm, "docs".into()), (Ecosystem::Pnpm, "web".into())]);
    }

    /// `cargo deny check licenses` reads no advisories, and a directory known
    /// only at run time is no directory of the repository.
    #[test]
    fn what_is_not_an_audit_of_the_repository_covers_nothing() {
        let yaml = "jobs:\n  a:\n    steps:\n      - run: cargo deny check licenses bans\n      - run: pnpm install\n      - run: pnpm audit\n        working-directory: ${{ runner.temp }}/demo\n      - uses: EmbarkStudios/cargo-deny-action@v2\n        with:\n          command: check bans\n";
        assert_eq!(coverage(yaml), Vec::new());
        let yaml = "jobs:\n  a:\n    steps:\n      - run: cargo deny --all-features check advisories\n";
        assert_eq!(coverage(yaml), vec![(Ecosystem::Cargo, ".".into())]);
    }
}
