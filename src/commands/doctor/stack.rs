//! The stack a repository builds on, side by side with the standard's: Rust's
//! edition and the promised toolchain, Node, pnpm, the actions, and when the
//! dependencies were last brought up to date.
//!
//! Not a rule: a repository a version behind on pnpm is not broken, and the
//! rules that do fail on it (`node`, `actions`) say so on their own. What the
//! stack adds is the view across repositories - who has fallen behind - and
//! the one habit no file shows: the stack is brought up to date at the start
//! of a version, once a week at most. Whether that happened is read from the
//! history, by the fact, not from anyone's memory of it.

use std::collections::BTreeMap;

use chrono::{DateTime, Duration, FixedOffset, Utc};
use regex::Regex;

use super::{Repo, actions, msrv, node};
use crate::generate::Contents;
use crate::model::Form;
use crate::templates;

/// How long a stack update holds: a version that starts within a week of the
/// last one does not need another.
const UPDATE_HOLDS_DAYS: i64 = 7;

/// What the standard builds on today, read from the templates lyrn writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Standard {
    pub edition: String,
    pub node: u32,
    /// `10.34.5`, as the templates pin it.
    pub pnpm: String,
    pub actions: BTreeMap<String, u32>,
}

impl Standard {
    pub fn read() -> Standard {
        let edition = Regex::new(r#"(?m)^edition\s*=\s*"(\d+)""#).expect("a valid pattern");
        let mut newest: Option<String> = None;
        for &form in Form::ALL {
            for source in templates::sources_for(form) {
                let Contents::Text(text) = source.contents else { continue };
                if !source.path.ends_with("Cargo.toml") {
                    continue;
                }
                for c in edition.captures_iter(text) {
                    if newest.as_deref().is_none_or(|n| c[1] > *n) {
                        newest = Some(c[1].to_string());
                    }
                }
            }
        }
        let toolchain = node::standard();
        Standard {
            edition: newest.expect("a template's Cargo.toml names its edition"),
            node: toolchain.node_major,
            pnpm: toolchain.package_manager.trim_start_matches("pnpm@").to_string(),
            actions: actions::standard_majors(),
        }
    }
}

/// One repository's stack.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Stack {
    /// The oldest edition among the manifests.
    pub edition: Option<String>,
    /// The `rust-version` promised.
    pub msrv: Option<String>,
    /// A toolchain pinned in `rust-toolchain.toml`, when one is.
    pub toolchain: Option<String>,
    /// The oldest Node a workflow sets up.
    pub node: Option<u32>,
    /// The oldest pnpm a `packageManager` pins.
    pub pnpm: Option<String>,
    /// The actions behind the standard's major, as `actions/checkout@v4`;
    /// `None` without workflows.
    pub actions_behind: Option<Vec<String>>,
    /// When the stack was last brought up to date; `None` outside git.
    pub updated: Option<Updated>,
}

/// The last stack update, and whether the last version kept the habit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Updated {
    /// The day of the last `chore(deps)` commit and how long ago it was; none
    /// when the history has never had one.
    pub last: Option<(String, i64)>,
    /// The newest version, when it started a week or more after the last
    /// update and shipped without one.
    pub skipped_by: Option<String>,
}

impl Stack {
    pub fn read(repo: &Repo, now: DateTime<Utc>) -> Stack {
        Stack {
            edition: edition(repo),
            msrv: msrv::MANIFESTS
                .iter()
                .filter(|m| repo.has_file(m) || repo.root.join(m).is_file())
                .filter_map(|m| msrv::declared_version(repo, m).ok().flatten())
                .max_by(|a, b| msrv::compare(a, b)),
            toolchain: toolchain(repo),
            node: oldest_node(repo),
            pnpm: oldest_pnpm(repo),
            actions_behind: (!repo.workflows.is_empty()).then(|| actions_behind(repo)),
            updated: repo.git.as_ref().filter(|g| g.has_commits()).map(|git| {
                let mut updates = dates(&git.run(&["log", "-E", "--grep=^(chore|build)\\(deps\\)", "--format=%cI"]).unwrap_or_default());
                // A project starts on the stack of the day it starts: its
                // first commit is an update too.
                updates.extend(dates(&git.run(&["log", "--max-parents=0", "--format=%cI"]).unwrap_or_default()));
                let tags = versions(
                    &git.run(&[
                        "for-each-ref",
                        "--sort=-creatordate",
                        "--format=%(refname:short)%09%(creatordate:iso-strict)",
                        "refs/tags/v*",
                    ])
                    .unwrap_or_default(),
                );
                updated(&updates, &tags, now)
            }),
        }
    }

    /// The cells of the stack that are behind the standard, by column name.
    pub fn behind(&self, standard: &Standard) -> Vec<&'static str> {
        let mut behind = Vec::new();
        if self.edition.as_ref().is_some_and(|e| e.as_str() < standard.edition.as_str()) {
            behind.push("edition");
        }
        if self.node.is_some_and(|n| n < standard.node) {
            behind.push("node");
        }
        if self.pnpm.as_ref().is_some_and(|p| version_cmp(p, &standard.pnpm).is_lt()) {
            behind.push("pnpm");
        }
        if self.actions_behind.as_ref().is_some_and(|a| !a.is_empty()) {
            behind.push("actions");
        }
        if self.updated.as_ref().is_some_and(|u| u.skipped_by.is_some() || u.last.is_none()) {
            behind.push("updated");
        }
        behind
    }
}

/// The oldest edition any manifest builds with, following
/// `edition.workspace = true` to the top.
fn edition(repo: &Repo) -> Option<String> {
    let top: Option<toml::Table> = repo.read_text("Cargo.toml").and_then(|t| toml::from_str(&t).ok());
    let workspace = top
        .as_ref()
        .and_then(|t| t.get("workspace")?.get("package")?.get("edition")?.as_str().map(str::to_string));
    let mut editions: Vec<String> = Vec::new();
    for manifest in repo.files_named("Cargo.toml") {
        if super::is_test_material(manifest) {
            continue;
        }
        let Some(table) = repo.read_text(manifest).and_then(|t| toml::from_str::<toml::Table>(&t).ok()) else {
            continue;
        };
        match table.get("package").and_then(|p| p.get("edition")) {
            Some(toml::Value::String(edition)) => editions.push(edition.clone()),
            Some(toml::Value::Table(_)) => editions.extend(workspace.clone()),
            // A package without an edition is on 2015, which nobody means.
            None if table.get("package").is_some() => editions.push("2015".to_string()),
            _ => {}
        }
    }
    if editions.is_empty() {
        return workspace;
    }
    editions.into_iter().min()
}

/// The channel `rust-toolchain.toml` or `rust-toolchain` pins, if either does.
fn toolchain(repo: &Repo) -> Option<String> {
    if let Some(text) = repo.read_text("rust-toolchain.toml") {
        let table: toml::Table = toml::from_str(&text).ok()?;
        return table.get("toolchain")?.get("channel")?.as_str().map(str::to_string);
    }
    repo.read_text("rust-toolchain").map(|t| t.trim().to_string()).filter(|t| !t.is_empty())
}

fn oldest_node(repo: &Repo) -> Option<u32> {
    repo.workflows
        .iter()
        .flat_map(|w| w.jobs())
        .flat_map(|j| j.steps)
        .filter(|s| s.uses.is_some_and(|u| u.starts_with("actions/setup-node@")))
        .filter_map(|s| s.input("node-version"))
        .filter_map(|v| node::major(&v))
        .min()
}

fn oldest_pnpm(repo: &Repo) -> Option<String> {
    repo.files_named("pnpm-lock.yaml")
        .filter(|lock| !super::is_test_material(lock))
        .filter_map(|lock| {
            let manifest = format!("{}package.json", lock.strip_suffix("pnpm-lock.yaml").unwrap_or_default());
            let json: serde_json::Value = serde_json::from_str(&repo.read_text(&manifest)?).ok()?;
            let pinned = json.get("packageManager")?.as_str()?;
            let version = pinned.strip_prefix("pnpm@")?;
            Some(version.split('+').next().unwrap_or(version).to_string())
        })
        .min_by(|a, b| version_cmp(a, b))
}

/// Every action a workflow uses at a major the standard has moved past.
fn actions_behind(repo: &Repo) -> Vec<String> {
    let standard = actions::standard_majors();
    let mut behind: Vec<String> = Vec::new();
    for workflow in &repo.workflows {
        for job in workflow.jobs() {
            for uses in job.uses.into_iter().chain(job.steps.iter().filter_map(|s| s.uses)) {
                let Some((action, version)) = uses.split_once('@') else { continue };
                let key = action.split('/').take(2).collect::<Vec<_>>().join("/");
                if let (Some(major), Some(&current)) = (actions::major_of(version), standard.get(&key))
                    && major < current
                    && !behind.iter().any(|b| b == uses)
                {
                    behind.push(uses.to_string());
                }
            }
        }
    }
    behind.sort();
    behind
}

/// `git log --format=%cI` read into moments, newest first as git prints them.
fn dates(log: &str) -> Vec<DateTime<FixedOffset>> {
    log.lines().filter_map(|l| DateTime::parse_from_rfc3339(l.trim()).ok()).collect()
}

/// `git for-each-ref` of the version tags, newest first.
fn versions(refs: &str) -> Vec<(String, DateTime<FixedOffset>)> {
    refs.lines()
        .filter_map(|line| {
            let (tag, date) = line.split_once('\t')?;
            Some((tag.trim().to_string(), DateTime::parse_from_rfc3339(date.trim()).ok()?))
        })
        .collect()
}

/// The habit, by the fact. The newest version's work began when the version
/// before it shipped; if the stack's last update was a week or more old by
/// then and none came while the version was built, it skipped one.
fn updated(updates: &[DateTime<FixedOffset>], tags: &[(String, DateTime<FixedOffset>)], now: DateTime<Utc>) -> Updated {
    let last = updates
        .iter()
        .max()
        .map(|d| (d.format("%Y-%m-%d").to_string(), (now - d.with_timezone(&Utc)).num_days()));
    let skipped_by = match tags {
        [(newest, shipped), (_, began), ..] => {
            let during = updates.iter().any(|d| d > began && d <= shipped);
            let before = updates.iter().filter(|d| *d <= began).max();
            let fresh = before.is_some_and(|d| *began - *d < Duration::days(UPDATE_HOLDS_DAYS));
            (!during && !fresh).then(|| newest.clone())
        }
        _ => None,
    };
    Updated { last, skipped_by }
}

/// Versions by their numbers: `10.9.0` is older than `10.34.5`.
pub fn version_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let parts = |v: &str| -> Vec<u64> { v.split('.').map(|p| p.parse().unwrap_or(0)).collect() };
    parts(a).cmp(&parts(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(text: &str) -> DateTime<FixedOffset> {
        DateTime::parse_from_rfc3339(text).unwrap()
    }

    fn now() -> DateTime<Utc> {
        at("2026-10-13T12:00:00+00:00").with_timezone(&Utc)
    }

    /// If this fails, the templates stopped saying what the stack reads.
    #[test]
    fn the_standard_is_read_from_the_templates() {
        let standard = Standard::read();
        assert!(standard.edition.as_str() >= "2024", "{standard:?}");
        assert!(standard.pnpm.split('.').count() == 3, "{standard:?}");
        assert!(standard.actions.contains_key("actions/checkout"), "{standard:?}");
    }

    #[test]
    fn a_version_started_on_a_fresh_stack_kept_the_habit() {
        // Updated three days before the version began.
        let updates = [at("2026-10-01T10:00:00+00:00")];
        let tags = [
            ("v1.2.0".to_string(), at("2026-10-09T10:00:00+00:00")),
            ("v1.1.0".to_string(), at("2026-10-04T10:00:00+00:00")),
        ];
        let u = updated(&updates, &tags, now());
        assert_eq!(u.skipped_by, None);
        assert_eq!(u.last, Some(("2026-10-01".to_string(), 12)));
    }

    #[test]
    fn a_version_that_updated_while_it_was_built_kept_the_habit() {
        let updates = [at("2026-10-05T10:00:00+00:00"), at("2026-08-01T10:00:00+00:00")];
        let tags = [
            ("v1.2.0".to_string(), at("2026-10-09T10:00:00+00:00")),
            ("v1.1.0".to_string(), at("2026-10-04T10:00:00+00:00")),
        ];
        assert_eq!(updated(&updates, &tags, now()).skipped_by, None);
    }

    #[test]
    fn a_version_started_on_a_stale_stack_without_updating_skipped_it() {
        // A week and a day old when the version began, none since.
        let updates = [at("2026-09-26T09:00:00+00:00")];
        let tags = [
            ("v1.2.0".to_string(), at("2026-10-09T10:00:00+00:00")),
            ("v1.1.0".to_string(), at("2026-10-04T10:00:00+00:00")),
        ];
        assert_eq!(updated(&updates, &tags, now()).skipped_by.as_deref(), Some("v1.2.0"));
        // Never updated at all is the same thing.
        assert_eq!(updated(&[], &tags, now()).skipped_by.as_deref(), Some("v1.2.0"));
        // The first version has nothing before it to measure from.
        assert_eq!(updated(&[], &tags[..1], now()).skipped_by, None);
    }

    #[test]
    fn versions_compare_by_number() {
        assert!(version_cmp("10.9.0", "10.34.5").is_lt());
        assert!(version_cmp("10.34.5", "10.34.5").is_eq());
        assert!(version_cmp("11.0.0", "10.34.5").is_gt());
    }

    #[test]
    fn the_cells_behind_are_named() {
        let standard = Standard {
            edition: "2024".into(),
            node: 22,
            pnpm: "10.34.5".into(),
            actions: BTreeMap::new(),
        };
        let stack = Stack {
            edition: Some("2021".into()),
            node: Some(20),
            pnpm: Some("10.12.0".into()),
            actions_behind: Some(vec!["actions/checkout@v4".into()]),
            updated: Some(Updated {
                last: Some(("2026-10-01".into(), 12)),
                skipped_by: Some("v1.2.0".into()),
            }),
            ..Stack::default()
        };
        assert_eq!(stack.behind(&standard), ["edition", "node", "pnpm", "actions", "updated"]);
        assert!(Stack::default().behind(&standard).is_empty());
    }
}
