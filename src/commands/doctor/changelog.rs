//! `changelog`: every release in CHANGELOG.md starts after a blank line.
//!
//! git-cliff writes the changelog one release at a time, and a body template
//! that ends without a blank line under `trim = true` glues each new release
//! to the last line of the one before. Markdown then reads the heading as part
//! of the list above it in some renderers, and the file reads as one block in
//! all of them. The repair is the text with the blank lines back; the cause is
//! the repository's cliff.toml, which the fix names.

use super::{Finding, Outcome, Repair, Repo, newline_of, plural};

const FILE: &str = "CHANGELOG.md";

pub fn check(repo: &Repo) -> Outcome {
    if !repo.has_file(FILE) {
        return Outcome::Skip("no CHANGELOG.md".to_string());
    }
    let Some(text) = repo.read_text(FILE) else {
        return Outcome::Found(vec![Finding::fail("CHANGELOG.md cannot be read as text")]);
    };
    let (glued, spaced) = space_releases(&text);
    let releases = text.lines().filter(|l| is_release(l)).count();
    if glued.is_empty() {
        return Outcome::Pass(match releases {
            0 => "no release yet".to_string(),
            n => format!("{} apart from what is above", plural(n, "release", "releases")),
        });
    }
    Outcome::Found(vec![
        Finding::fail(format!(
            "CHANGELOG.md: {} straight after the line above - the first, `{}`, at line {}",
            plural(glued.len(), "release starts", "releases start"),
            glued[0].1,
            glued[0].0
        ))
        .fix("end the `body` of cliff.toml with an empty line, or open it with one as the line's cliff.toml does, so the next release is written apart too")
        .repair(Repair::new(FILE, spaced, "puts a blank line before every release")),
    ])
}

/// A release heading: `## [1.2.0] - ...`, `## 🎉 [1.2.0]`, `## [unreleased]`.
fn is_release(line: &str) -> bool {
    line.strip_prefix("## ").is_some_and(|rest| rest.contains('['))
}

/// The releases that follow a line without a blank one between, as line
/// number and heading, and the text with a blank line before each.
fn space_releases(text: &str) -> (Vec<(usize, String)>, String) {
    let newline = newline_of(text);
    let mut glued = Vec::new();
    let mut out = String::with_capacity(text.len());
    let mut previous: Option<&str> = None;
    for (i, line) in text.split_inclusive('\n').enumerate() {
        let bare = line.trim_end_matches(['\n', '\r']);
        if is_release(bare) && previous.is_some_and(|p| !p.trim().is_empty()) {
            glued.push((i + 1, bare.to_string()));
            out.push_str(newline);
        }
        out.push_str(line);
        previous = Some(bare);
    }
    (glued, out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_release_glued_to_the_list_above_gets_its_blank_line() {
        let text = "# Changelog\n\n## [0.2.0] - 2026-10-20\n\n### Features\n- Two\n## [0.1.0] - 2026-10-19\n- One\n";
        let (glued, spaced) = space_releases(text);
        assert_eq!(glued, vec![(7, "## [0.1.0] - 2026-10-19".to_string())]);
        assert_eq!(
            spaced,
            "# Changelog\n\n## [0.2.0] - 2026-10-20\n\n### Features\n- Two\n\n## [0.1.0] - 2026-10-19\n- One\n"
        );
        assert!(space_releases(&spaced).0.is_empty(), "the repair repairs");
    }

    #[test]
    fn a_crlf_changelog_stays_crlf() {
        let (glued, spaced) = space_releases("- Two\r\n## 🎉 [0.1.0] - 2026-10-19\r\n");
        assert_eq!(glued.len(), 1);
        assert_eq!(spaced, "- Two\r\n\r\n## 🎉 [0.1.0] - 2026-10-19\r\n");
    }

    /// A heading of the file's own - not a release - is not a release.
    #[test]
    fn only_releases_are_held_apart() {
        assert!(space_releases("intro\n## Notes\n").0.is_empty());
        assert!(!space_releases("intro\n## [unreleased]\n").0.is_empty());
    }
}
