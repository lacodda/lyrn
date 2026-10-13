//! `actions`: no GitHub Action or workflow command GitHub has retired.
//!
//! Two kinds of finding. A dead action stops a workflow from running at all,
//! or runs one nobody maintains: those are listed here, each with why. An
//! action behind the standard still runs, on a major the line has moved past;
//! the line takes every major as it comes, so that is a failure too. What the
//! standard is on is read from the templates lyrn writes, so that moving a
//! template to a new major moves this rule with it.

use std::collections::BTreeMap;

use regex::Regex;

use super::{Finding, Outcome, Repo, plural, some_of};
use crate::generate::Contents;
use crate::model::Form;
use crate::templates;

/// An action that no longer works, or no longer should be used.
struct Dead {
    /// The action, or an organisation as `owner/`.
    action: &'static str,
    /// Dead below this major; `None` for every version.
    below: Option<u32>,
    why: &'static str,
    instead: &'static str,
}

const DEAD: &[Dead] = &[
    Dead {
        action: "actions/create-release",
        below: None,
        why: "archived in 2021, on a retired Node runtime",
        instead: "`gh release create` in a run step",
    },
    Dead {
        action: "actions/upload-release-asset",
        below: None,
        why: "archived in 2021, on a retired Node runtime",
        instead: "`gh release upload` in a run step",
    },
    Dead {
        action: "actions-rs/",
        below: None,
        why: "the actions-rs actions were archived in 2023",
        instead: "dtolnay/rust-toolchain and plain cargo commands",
    },
    Dead {
        action: "actions/upload-artifact",
        below: Some(4),
        why: "v1-v3 stopped working when GitHub retired them in January 2025",
        instead: "v4 or later",
    },
    Dead {
        action: "actions/download-artifact",
        below: Some(4),
        why: "v1-v3 stopped working when GitHub retired them in January 2025",
        instead: "v4 or later",
    },
    Dead {
        action: "actions/cache",
        below: Some(3),
        why: "v1-v2 stopped working when the old cache service shut down in February 2025",
        instead: "v4 or later",
    },
    Dead {
        action: "actions/upload-pages-artifact",
        below: Some(3),
        why: "v1-v2 upload through the artifact actions retired in January 2025",
        instead: "v3 or later",
    },
    Dead {
        action: "actions/deploy-pages",
        below: Some(4),
        why: "v1-v3 read artifacts only the retired artifact actions wrote",
        instead: "v4 or later",
    },
];

/// Workflow commands GitHub has turned off or deprecated.
const COMMANDS: &[(&str, &str, &str)] = &[
    ("::set-output", "deprecated since October 2022", "echo \"name=value\" >> \"$GITHUB_OUTPUT\""),
    ("::save-state", "deprecated since October 2022", "echo \"name=value\" >> \"$GITHUB_STATE\""),
    (
        "::set-env",
        "disabled since November 2020: the step fails",
        "echo \"NAME=value\" >> \"$GITHUB_ENV\"",
    ),
    (
        "::add-path",
        "disabled since November 2020: the step fails",
        "echo \"/the/dir\" >> \"$GITHUB_PATH\"",
    ),
];

pub fn check(repo: &Repo) -> Outcome {
    if repo.workflows.is_empty() && repo.broken_workflows.is_empty() {
        return Outcome::Skip("no GitHub workflows".to_string());
    }
    let standard = standard_majors();
    let mut findings = Vec::new();

    for (file, reason) in &repo.broken_workflows {
        findings
            .push(Finding::fail(format!("{file} does not parse, so GitHub runs none of it: {reason}")).fix("fix the YAML; `actionlint` points at the line"));
    }

    // One finding per action and version, naming every file it is in: the
    // same `checkout@v4` in ten jobs is one thing to change.
    let mut used: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
    let mut commands: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    let mut steps = 0;
    for workflow in &repo.workflows {
        for job in workflow.jobs() {
            let uses = job.uses.into_iter().chain(job.steps.iter().filter_map(|s| s.uses));
            for reference in uses {
                steps += 1;
                if let Some((action, version)) = reference.split_once('@') {
                    let files = used.entry((action.to_string(), version.to_string())).or_default();
                    if !files.iter().any(|f| f == workflow.name()) {
                        files.push(workflow.name().to_string());
                    }
                }
            }
            for script in job.steps.iter().filter_map(|s| s.run) {
                for (command, _, _) in COMMANDS {
                    if script.contains(command) {
                        let files = commands.entry(command).or_default();
                        if !files.iter().any(|f| f == workflow.name()) {
                            files.push(workflow.name().to_string());
                        }
                    }
                }
            }
        }
    }

    for ((action, version), files) in &used {
        if action.starts_with("./") || action.starts_with("docker://") {
            continue;
        }
        let major = major_of(version).or_else(|| pinned_major(repo, version));
        let place = some_of(files, 3);
        if let Some(dead) = DEAD
            .iter()
            .find(|d| is(action, d.action) && d.below.is_none_or(|below| major.is_some_and(|m| m < below)))
        {
            findings.push(Finding::fail(format!("{action}@{version} in {place}: {}", dead.why)).fix(format!("use {}", dead.instead)));
            continue;
        }
        // A reusable workflow is named by its path; the action is the repository.
        let key = action.split('/').take(2).collect::<Vec<_>>().join("/");
        if let (Some(major), Some(&current)) = (major, standard.get(&key))
            && major < current
        {
            findings.push(
                Finding::fail(format!("{action}@{version} in {place}: the standard is on v{current}"))
                    .fix(format!("move to {key}@v{current}, as the templates lyrn writes are")),
            );
        }
    }

    for (command, files) in &commands {
        let (_, why, instead) = COMMANDS.iter().find(|(c, _, _)| c == command).expect("listed above");
        findings.push(Finding::fail(format!("`{command}` in {}: {why}", some_of(files, 3))).fix(format!("write `{instead}`")));
    }

    Outcome::from_findings(findings, || {
        format!(
            "{} in {}, none retired or behind the standard",
            plural(steps, "action", "actions"),
            plural(repo.workflows.len(), "workflow", "workflows")
        )
    })
}

/// Whether `action` is the one a rule names: exactly, or within an
/// organisation named as `owner/`.
fn is(action: &str, named: &str) -> bool {
    if named.ends_with('/') {
        action.starts_with(named)
    } else {
        action == named || action.starts_with(&format!("{named}/"))
    }
}

/// `v4`, `v4.1.2` and `4` are major 4; a branch or a commit has none.
pub(super) fn major_of(version: &str) -> Option<u32> {
    let digits = version.strip_prefix('v').unwrap_or(version);
    let end = digits.find(|c: char| !c.is_ascii_digit()).unwrap_or(digits.len());
    if end == 0 || (end < digits.len() && !digits[end..].starts_with('.')) {
        return None;
    }
    digits[..end].parse().ok()
}

/// A commit pin says which version it stands for in a comment, as
/// `uses: actions/checkout@<sha> # v4.2.2`; the parser drops comments, so the
/// line is looked up in the text.
fn pinned_major(repo: &Repo, sha: &str) -> Option<u32> {
    if sha.len() < 7 || !sha.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let comment = Regex::new(r"#\s*(v?\d+(?:\.\d+)*)").expect("a valid pattern");
    repo.workflows
        .iter()
        .flat_map(|w| w.text.lines())
        .find(|line| line.contains(sha))
        .and_then(|line| comment.captures(line))
        .and_then(|c| major_of(&c[1]))
}

/// The major of every action the built-in templates use: what the standard is
/// on today.
pub fn standard_majors() -> BTreeMap<String, u32> {
    let uses = Regex::new(r"uses:\s*([\w.-]+/[\w.-]+)[^@\s]*@v(\d+)").expect("a valid pattern");
    let mut majors: BTreeMap<String, u32> = BTreeMap::new();
    for &form in Form::ALL {
        for source in templates::sources_for(form) {
            if !source.path.starts_with(".github/workflows/") {
                continue;
            }
            let Contents::Text(text) = source.contents else { continue };
            for captures in uses.captures_iter(text) {
                let Ok(major) = captures[2].parse::<u32>() else { continue };
                let entry = majors.entry(captures[1].to_string()).or_insert(major);
                *entry = (*entry).max(major);
            }
        }
    }
    majors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_version_has_a_major_and_a_branch_has_none() {
        assert_eq!(major_of("v4"), Some(4));
        assert_eq!(major_of("v4.1.2"), Some(4));
        assert_eq!(major_of("4"), Some(4));
        assert_eq!(major_of("main"), None);
        assert_eq!(major_of("stable"), None);
        assert_eq!(major_of("v4beta"), None);
    }

    /// The standard is what the templates are on, not a number written here:
    /// if this fails, a template stopped using actions the rule relies on.
    #[test]
    fn the_standard_is_read_from_the_templates() {
        let majors = standard_majors();
        for action in ["actions/checkout", "actions/setup-node", "pnpm/action-setup", "Swatinem/rust-cache"] {
            assert!(majors.contains_key(action), "the templates no longer use {action}: {majors:?}");
        }
        // Nothing the templates use may be dead by the rule's own list.
        for (action, major) in &majors {
            assert!(
                !DEAD.iter().any(|d| is(action, d.action) && d.below.is_none_or(|b| *major < b)),
                "the templates use {action}@v{major}, which this rule calls dead"
            );
        }
    }

    #[test]
    fn an_organisation_covers_its_actions() {
        assert!(is("actions-rs/toolchain", "actions-rs/"));
        assert!(is("actions/cache", "actions/cache"));
        assert!(is("actions/cache/restore", "actions/cache"));
        assert!(!is("actions/cache-extra", "actions/cache"));
    }
}
