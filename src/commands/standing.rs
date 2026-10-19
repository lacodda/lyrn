//! Where a repository stands, as git and GitHub see it: the branch and how it
//! sits against its upstream, what the work tree holds, the last release and
//! what has been committed since, and what the CI said about a commit.
//!
//! `status` and `stage` say these things, and they say them from here, so the
//! two can never read the same history two ways.

use chrono::{Duration, NaiveDate};

use crate::commands::doctor::Git;
use crate::commands::doctor::stack::Updated;
use crate::template::github::{self, Verdict};

/// How long a stack update holds, as the doctor's stack has it.
const UPDATE_HOLDS_DAYS: i64 = 7;

pub struct Standing {
    pub branch: Option<String>,
    /// `origin/main`, when the branch tracks one.
    pub upstream: Option<String>,
    pub ahead: usize,
    pub behind: usize,
    pub changed: usize,
    pub untracked: usize,
    /// The commit checked out, in full.
    pub head: Option<String>,
    pub release: Option<Release>,
}

/// The newest version tag behind the commit checked out.
pub struct Release {
    pub tag: String,
    pub day: String,
    pub commit: String,
    /// Commit subjects since the tag, newest first.
    pub since: Vec<String>,
}

impl Standing {
    pub fn read(git: &Git) -> Standing {
        let line = |args: &[&str]| git.run(args).map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        let branch = line(&["symbolic-ref", "--quiet", "--short", "HEAD"]);
        let upstream = line(&["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"]);
        let (behind, ahead) = upstream
            .as_ref()
            .and_then(|_| line(&["rev-list", "--left-right", "--count", "@{u}...HEAD"]))
            .and_then(|counts| {
                let mut parts = counts.split_whitespace().map(|n| n.parse::<usize>().unwrap_or(0));
                Some((parts.next()?, parts.next()?))
            })
            .unwrap_or((0, 0));
        let status = git.run(&["status", "--porcelain"]).unwrap_or_default();
        let untracked = status.lines().filter(|l| l.starts_with("??")).count();
        let changed = status.lines().filter(|l| !l.trim().is_empty()).count() - untracked;
        let release = line(&["describe", "--tags", "--abbrev=0", "--match", "v[0-9]*"]).map(|tag| {
            let since = git
                .run(&["log", "--format=%s", &format!("{tag}..HEAD")])
                .unwrap_or_default()
                .lines()
                .map(str::to_string)
                .collect();
            Release {
                day: line(&["log", "-1", "--format=%cs", &tag]).unwrap_or_default(),
                commit: line(&["rev-list", "-n", "1", &tag]).unwrap_or_default(),
                since,
                tag,
            }
        });
        Standing {
            branch,
            upstream,
            ahead,
            behind,
            changed,
            untracked,
            head: line(&["rev-parse", "HEAD"]),
            release,
        }
    }

    /// The branch against its upstream, in a few words.
    pub fn branch_text(&self) -> String {
        let branch = self.branch.as_deref().map_or("no branch (a detached commit)".to_string(), |b| b.to_string());
        let Some(upstream) = &self.upstream else {
            return format!("{branch}, tracking nothing - never pushed");
        };
        match (self.ahead, self.behind) {
            (0, 0) => format!("{branch}, level with {upstream}"),
            (ahead, 0) => format!("{branch}, {} ahead of {upstream} - not pushed", commits(ahead)),
            (0, behind) => format!("{branch}, {} behind {upstream}", commits(behind)),
            (ahead, behind) => format!("{branch}, {} ahead of {upstream} and {behind} behind", commits(ahead)),
        }
    }

    pub fn tree_text(&self) -> String {
        match (self.changed, self.untracked) {
            (0, 0) => "clean".to_string(),
            (changed, 0) => format!("{} changed", files(changed)),
            (0, untracked) => format!("{} untracked", files(untracked)),
            (changed, untracked) => format!("{} changed, {untracked} untracked", files(changed)),
        }
    }

    pub fn clean(&self) -> bool {
        self.changed == 0 && self.untracked == 0
    }
}

impl Release {
    /// The release and what came after it: "v2.12.0 on 2026-10-13, 3
    /// commits since: 2 feat, 1 fix".
    pub fn text(&self) -> String {
        let at = format!("{} on {}", self.tag, self.day);
        if self.since.is_empty() {
            return format!("{at}, nothing committed since");
        }
        let mut kinds: Vec<(String, usize)> = Vec::new();
        for subject in &self.since {
            let kind = subject
                .split([':', '(', '!'])
                .next()
                .filter(|k| !k.is_empty() && k.chars().all(|c| c.is_ascii_lowercase()))
                .unwrap_or("other")
                .to_string();
            match kinds.iter_mut().find(|(k, _)| *k == kind) {
                Some((_, n)) => *n += 1,
                None => kinds.push((kind, 1)),
            }
        }
        let kinds: Vec<String> = kinds.iter().map(|(k, n)| format!("{n} {k}")).collect();
        format!("{at}, {} since: {}", commits(self.since.len()), kinds.join(", "))
    }
}

/// What GitHub's CI said about a commit, in a few words.
pub fn ci_text(slug: &str, commit: &str) -> String {
    let short = &commit[..commit.len().min(7)];
    match github::ci(slug, commit) {
        Ok(Verdict::Green) => format!("green on {short}"),
        Ok(Verdict::NoCi) => format!("nothing ran on {short}"),
        Ok(Verdict::Running(names)) => format!("still running on {short}: {}", names.join(", ")),
        Ok(Verdict::Failed(names)) => format!("red on {short}: {}", names.join(", ")),
        Err(reason) => format!("could not ask GitHub - {reason}"),
    }
}

/// When the stack was last brought up to date, and whether that still holds.
pub fn stack_text(updated: &Updated, today: NaiveDate) -> (String, bool) {
    let Some((day, days)) = &updated.last else {
        return ("never brought up to date - no `chore(deps)` commit in the history".to_string(), true);
    };
    let ago = match days {
        0 => "today".to_string(),
        1 => "yesterday".to_string(),
        n => format!("{n} days ago"),
    };
    let until = NaiveDate::parse_from_str(day, "%Y-%m-%d")
        .map(|d| d + Duration::days(UPDATE_HOLDS_DAYS))
        .unwrap_or(today);
    if until <= today {
        (format!("updated {ago}, on {day} - due since {until}"), true)
    } else {
        (format!("updated {ago}, on {day} - holds until {until}"), false)
    }
}

fn commits(n: usize) -> String {
    if n == 1 { "1 commit".to_string() } else { format!("{n} commits") }
}

fn files(n: usize) -> String {
    if n == 1 { "1 file".to_string() } else { format!("{n} files") }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_came_after_a_release_is_counted_by_kind() {
        let release = Release {
            tag: "v1.2.0".into(),
            day: "2026-10-13".into(),
            commit: "abc".into(),
            since: vec![
                "feat(ci): run here".into(),
                "fix: a thing".into(),
                "feat!: break".into(),
                "Merge branch x".into(),
            ],
        };
        assert_eq!(release.text(), "v1.2.0 on 2026-10-13, 4 commits since: 2 feat, 1 fix, 1 other");
    }

    #[test]
    fn an_update_holds_for_a_week() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 19).unwrap();
        let updated = |day: &str, days: i64| Updated {
            last: Some((day.to_string(), days)),
            skipped_by: None,
        };
        assert_eq!(
            stack_text(&updated("2026-10-13", 6), today),
            ("updated 6 days ago, on 2026-10-13 - holds until 2026-10-20".to_string(), false)
        );
        assert_eq!(
            stack_text(&updated("2026-10-12", 7), today),
            ("updated 7 days ago, on 2026-10-12 - due since 2026-10-19".to_string(), true)
        );
        assert!(stack_text(&Updated { last: None, skipped_by: None }, today).1);
    }
}
