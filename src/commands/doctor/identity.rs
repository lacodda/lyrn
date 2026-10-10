//! `identity`: this machine commits as its global identity, and keeps
//! assistant files out of every repository.
//!
//! Both are settings of the machine rather than of the repository, which is
//! why this rule only warns. An identity set inside one repository is how a
//! wrong name reaches a public history: the global one is right everywhere
//! else, and nobody looks at the local one again. And a global excludes file
//! is the one place that keeps an assistant's files out of every repository
//! at once, including the ones whose `.gitignore` has never heard of them.

use std::path::PathBuf;

use super::ai::normalize_pattern;
use super::git::{Identity, global_config};
use super::{Finding, Outcome, Repo, some_of};
use crate::templates::community;

pub fn check(repo: &Repo) -> Outcome {
    if repo.on_ci {
        return Outcome::Skip("a CI runner has no identity or excludes file of its own".to_string());
    }
    let mut findings = Vec::new();

    let global = match (global_config("user.name"), global_config("user.email")) {
        (Some(name), Some(email)) => Some(Identity::new(name, email)),
        _ => None,
    };
    let effective = match &repo.git {
        Some(git) => match (git.config("user.name"), git.config("user.email")) {
            (Some(name), Some(email)) => Some(Identity::new(name, email)),
            _ => None,
        },
        None => global.clone(),
    };
    let origin = || {
        repo.git
            .as_ref()
            .and_then(|g| g.config_origin("user.name").or_else(|| g.config_origin("user.email")))
            .map(|o| o.trim_start_matches("file:").to_string())
            .unwrap_or_else(|| "the repository's config".to_string())
    };
    match (&global, &effective) {
        (_, None) => findings.push(
            Finding::fail("no git identity: commits here would carry a name git guesses from the machine")
                .fix("`git config --global user.name \"Your Name\"` and `git config --global user.email you@example.com`"),
        ),
        (None, Some(effective)) => findings.push(
            Finding::warn(format!(
                "commits here are signed `{effective}`, from {}, and there is no global identity",
                origin()
            ))
            .fix("set it once, globally: `git config --global user.name` and `user.email`"),
        ),
        (Some(global), Some(effective)) if !global.same_as(effective) => findings.push(
            Finding::warn(format!(
                "commits here are signed `{effective}`, from {}; the global identity is `{global}`",
                origin()
            ))
            .fix("`git config --unset user.name` and `git config --unset user.email` here, unless this repository really is somebody else's"),
        ),
        _ => {}
    }

    let excludes = excludes_file();
    match excludes.as_ref().and_then(|path| std::fs::read_to_string(path).ok().map(|text| (path, text))) {
        None => findings.push(
            Finding::warn("no global excludes file: an assistant's file in any repository on this machine is one `git add -A` from its history")
                .fix("`git config --global core.excludesFile ~/.gitignore_global`, and end that file with the standard's assistant block"),
        ),
        Some((path, text)) => {
            let kept: Vec<String> = text.lines().map(normalize_pattern).collect();
            let missing: Vec<String> = community::assistant_patterns()
                .into_iter()
                .filter(|p| !kept.contains(&normalize_pattern(p)))
                .map(str::to_string)
                .collect();
            if !missing.is_empty() {
                findings.push(
                    Finding::warn(format!(
                        "the global excludes file {} does not keep out {}",
                        path.display(),
                        some_of(&missing, 4)
                    ))
                    .fix("add them: the standard's block ends every `lyrn new` project's .gitignore"),
                );
            }
        }
    }

    Outcome::from_findings(findings, || {
        let who = effective.map(|e| e.to_string()).unwrap_or_default();
        format!("commits as `{who}`, the global identity; the global excludes file keeps assistant files out")
    })
}

/// The global excludes file: the one `core.excludesFile` names, else git's
/// default under the XDG config directory.
fn excludes_file() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from);
    if let Some(configured) = global_config("core.excludesFile") {
        return Some(match configured.strip_prefix("~/") {
            Some(rest) => home?.join(rest),
            None => PathBuf::from(configured),
        });
    }
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| home.map(|h| h.join(".config")))?;
    Some(config.join("git").join("ignore"))
}
