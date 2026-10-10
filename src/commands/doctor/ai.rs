//! `ai-traces`: no coding assistant's files or stamps in the repository or its
//! history.
//!
//! An assistant's instructions describe one person's way of working on one
//! machine; what anyone else needs is in CONTRIBUTING.md and llms.txt. And an
//! assistant is a tool, not an author: a `Co-authored-by` trailer naming one,
//! or a "Generated with" line, puts it in the public history for good. The
//! files are the ones every generated `.gitignore` keeps out - one list.

use std::collections::BTreeSet;

use super::git::{Git, Identity};
use super::{Finding, Outcome, Repo, plural, some_of};
use crate::templates::community;

pub fn check(repo: &Repo) -> Outcome {
    let patterns = community::assistant_patterns();
    let mut findings = Vec::new();

    let gitignore = repo.read_text(".gitignore").unwrap_or_default();
    let kept_out: BTreeSet<String> = gitignore.lines().map(normalize_pattern).collect();
    let missing: Vec<String> = patterns
        .iter()
        .filter(|p| !kept_out.contains(&normalize_pattern(p)))
        .map(|p| p.to_string())
        .collect();
    if !missing.is_empty() {
        let what = if gitignore.is_empty() {
            "no .gitignore keeps assistant files out".to_string()
        } else {
            format!(".gitignore does not keep out {}", some_of(&missing, 4))
        };
        findings.push(
            Finding::fail(format!("{what}: a clone on a machine without a global excludes file commits them"))
                .fix("end .gitignore with the standard's block - every `lyrn new` project's .gitignore ends with it"),
        );
    }

    let Some(git) = &repo.git else {
        return Outcome::from_findings(findings, || "not a git repository; .gitignore keeps assistant files out".to_string());
    };

    // A file of that name the repository ships on purpose - a briefing it
    // hands to the projects that use it - is said to be one in .gitignore,
    // by name, after the block: `!registry/files/AGENTS.md`.
    let tracked: Vec<String> = repo.files.iter().filter(|f| patterns.iter().any(|p| matches(p, f))).cloned().collect();
    let (shipped, tracked): (Vec<String>, Vec<String>) = tracked.into_iter().partition(|path| un_ignored(git, path));
    if !tracked.is_empty() {
        findings.push(Finding::fail(format!("tracked: {}", some_of(&tracked, 4))).fix(format!(
            "`git rm -r --cached {}`, then commit; the files stay on disk\nif one is shipped on purpose, un-ignore it by name in .gitignore: `!{}`",
            tracked.join(" "),
            tracked[0]
        )));
    }

    if git.has_commits() {
        let added = git
            .run(&["log", "--all", "--no-renames", "--diff-filter=A", "--format=", "--name-only"])
            .unwrap_or_default();
        let past: BTreeSet<String> = added
            .lines()
            .map(str::trim)
            .filter(|p| !p.is_empty() && !tracked.iter().chain(&shipped).any(|t| t == p))
            .filter(|p| patterns.iter().any(|pattern| matches(pattern, p)))
            .map(str::to_string)
            .collect();
        if !past.is_empty() {
            let past: Vec<String> = past.into_iter().collect();
            findings.push(
                Finding::warn(format!("in the history: {}", some_of(&past, 4)))
                    .fix("`git filter-repo --invert-paths --path <file>` - a rewrite: once pushed, every clone has to be replaced"),
            );
        }

        let stamped = stamped_commits(&git.run(&["log", "--all", "--format=%h%x1f%B%x1e"]).unwrap_or_default());
        if !stamped.is_empty() {
            findings.push(
                Finding::warn(format!(
                    "{} an assistant's stamp - a co-author trailer or a \"Generated with\" line: {}",
                    plural(stamped.len(), "commit carries", "commits carry"),
                    some_of(&stamped, 4)
                ))
                .fix("rewrite the messages with `git filter-repo --message-callback`; a commit-msg hook keeps new ones out"),
            );
        }
    }

    Outcome::from_findings(findings, || {
        "no assistant files tracked or in the history, no stamps, .gitignore keeps them out".to_string()
    })
}

/// Whether the repository's own ignore rules let `path` in by name: the
/// pattern that decides it is a negation.
fn un_ignored(git: &Git, path: &str) -> bool {
    let Some(verdict) = git.run(&["check-ignore", "--no-index", "--verbose", "--", path]) else {
        return false;
    };
    // `.gitignore:14:!registry/files/AGENTS.md<TAB>registry/files/AGENTS.md`;
    // a source on Windows starts with a drive, `C:/Users/me/.gitignore:3:`.
    let source_and_pattern = verdict.split('\t').next().unwrap_or_default();
    let bytes = source_and_pattern.as_bytes();
    let without_drive = if bytes.len() > 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' && bytes[2] == b'/' {
        &source_and_pattern[2..]
    } else {
        source_and_pattern
    };
    without_drive.splitn(3, ':').nth(2).is_some_and(|pattern| pattern.starts_with('!'))
}

/// The commits whose message carries an assistant's stamp, from
/// `git log --format=%h%x1f%B%x1e`.
fn stamped_commits(log: &str) -> Vec<String> {
    log.split('\x1e')
        .filter_map(|record| {
            let (hash, message) = record.trim_start().split_once('\x1f')?;
            stamped(message).then(|| hash.trim().to_string())
        })
        .collect()
}

fn stamped(message: &str) -> bool {
    message.lines().any(|line| {
        let lower = line.trim().to_lowercase();
        if let Some(value) = lower.strip_prefix("co-authored-by:") {
            return Identity::parse(value).is_some_and(|who| is_assistant(&who));
        }
        lower.contains("generated with [claude code]")
            || lower.contains("generated with claude")
            || lower.starts_with("claude-session:")
            || lower.contains("claude.ai/code/session_")
    })
}

/// Whether an identity is a coding assistant's rather than a person's.
pub fn is_assistant(who: &Identity) -> bool {
    const NAMES: &[&str] = &[
        "claude",
        "claude code",
        "copilot",
        "github copilot",
        "cursor agent",
        "codex",
        "gemini",
        "devin",
        "aider",
    ];
    let name = who.name.trim().to_lowercase();
    let email = who.email.to_lowercase();
    NAMES.contains(&name.as_str())
        || name.starts_with("claude ")
        || email.ends_with("@anthropic.com")
        || email == "cursoragent@cursor.com"
        || email.ends_with("+copilot@users.noreply.github.com")
        || email.contains("devin-ai-integration")
}

/// A pattern as written in an ignore file, for comparing: `/CLAUDE.md`,
/// `**/CLAUDE.md` and `CLAUDE.md` keep out the same file at the top.
pub fn normalize_pattern(line: &str) -> String {
    let line = line.trim();
    let line = line.strip_prefix("**/").unwrap_or(line);
    let line = line.strip_prefix('/').unwrap_or(line);
    line.trim_end_matches('/').to_string()
}

/// Whether `path` is what an ignore pattern of the block keeps out: a name
/// anywhere in the tree, a directory anywhere, or a path from the top when
/// the pattern has a slash inside.
fn matches(pattern: &str, path: &str) -> bool {
    let dir_only = pattern.ends_with('/');
    let name = pattern.trim_end_matches('/');
    if name.contains('/') {
        return path == name || path.starts_with(&format!("{name}/"));
    }
    let parts: Vec<&str> = path.split('/').collect();
    let candidates = if dir_only { &parts[..parts.len().saturating_sub(1)] } else { &parts[..] };
    candidates.contains(&name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pattern_keeps_out_what_git_would() {
        assert!(matches("CLAUDE.md", "CLAUDE.md"));
        assert!(matches("CLAUDE.md", "crates/core/CLAUDE.md"));
        assert!(matches(".claude/", ".claude/settings.json"));
        assert!(matches(".claude/", "app/.claude/commands/x.md"));
        assert!(!matches(".claude/", ".claude"), "a file is not the directory");
        assert!(matches(".github/copilot-instructions.md", ".github/copilot-instructions.md"));
        assert!(!matches(".github/copilot-instructions.md", "docs/.github/copilot-instructions.md"));
        assert!(!matches("AGENTS.md", "docs/AGENTS.md.bak"));
    }

    #[test]
    fn an_ignore_line_is_compared_by_what_it_keeps_out() {
        assert_eq!(normalize_pattern("/CLAUDE.md"), "CLAUDE.md");
        assert_eq!(normalize_pattern("**/.claude/"), ".claude");
        assert_eq!(normalize_pattern(".claude"), ".claude");
    }

    #[test]
    fn a_stamp_is_a_trailer_naming_an_assistant_or_a_generated_line() {
        assert!(stamped("feat: x\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>\n"));
        assert!(stamped("feat: x\n\n🤖 Generated with [Claude Code](https://claude.com/claude-code)\n"));
        assert!(stamped("fix: y\n\nClaude-Session: https://claude.ai/code/session_abc\n"));
        assert!(stamped("fix: y\n\nCo-authored-by: Copilot <198982749+Copilot@users.noreply.github.com>\n"));
        // A person as co-author is authorship, which `authors` looks at.
        assert!(!stamped("fix: y\n\nCo-authored-by: Ada Lovelace <ada@example.com>\n"));
        assert!(!stamped("docs: explain how claude-like tools are kept out\n"));
    }

    #[test]
    fn the_stamped_commits_are_named_by_hash() {
        let log = "abc1234\x1ffeat: one\n\nCo-Authored-By: Claude <noreply@anthropic.com>\n\x1e\ndef5678\x1ffix: two\n\x1e\n";
        assert_eq!(stamped_commits(log), vec!["abc1234".to_string()]);
    }
}
