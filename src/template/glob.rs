//! The gitignore-style patterns cargo-generate templates list files with.
//!
//! `ignore`, `include` and `exclude` in `cargo-generate.toml`, and the lines
//! of `.genignore`, all use them: a name without a slash matches at any depth,
//! one with a slash is anchored at the template's root, a trailing slash
//! matches only a directory, and `*`, `?` and `**` mean what they mean in
//! `.gitignore`. A pattern that names a directory takes everything under it.

use regex::Regex;

/// A compiled set of patterns.
#[derive(Debug, Clone)]
pub struct Patterns {
    compiled: Vec<Regex>,
}

impl Patterns {
    /// Compile `patterns`, skipping blanks and `#` comments.
    ///
    /// A negated pattern is refused rather than read as a positive one: `!x`
    /// taken literally would do the opposite of what it says.
    pub fn new<S: AsRef<str>>(patterns: &[S]) -> Result<Self, String> {
        let mut compiled = Vec::new();
        for pattern in patterns {
            let pattern = pattern.as_ref().trim();
            if pattern.is_empty() || pattern.starts_with('#') {
                continue;
            }
            if pattern.starts_with('!') {
                return Err(format!(
                    "`{pattern}` is a negated pattern, which lyrn does not read; list what to match instead"
                ));
            }
            compiled.push(Regex::new(&to_regex(pattern)).map_err(|e| format!("`{pattern}` is not a pattern lyrn can read: {e}"))?);
        }
        Ok(Self { compiled })
    }

    /// Whether `path` - relative, `/`-separated - is matched by any pattern.
    pub fn matches(&self, path: &str) -> bool {
        self.compiled.iter().any(|r| r.is_match(path))
    }

    pub fn is_empty(&self) -> bool {
        self.compiled.is_empty()
    }
}

fn to_regex(pattern: &str) -> String {
    let directory_only = pattern.ends_with('/');
    let body = pattern.trim_end_matches('/');
    // A slash anywhere but at the end anchors the pattern at the root.
    let anchored = body.contains('/');
    let body = body.trim_start_matches('/');

    let mut out = String::new();
    let chars: Vec<char> = body.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '*' if chars.get(i + 1) == Some(&'*') => {
                if chars.get(i + 2) == Some(&'/') {
                    out.push_str("(?:.*/)?");
                    i += 3;
                } else {
                    out.push_str(".*");
                    i += 2;
                }
            }
            '*' => {
                out.push_str("[^/]*");
                i += 1;
            }
            '?' => {
                out.push_str("[^/]");
                i += 1;
            }
            c => {
                out.push_str(&regex::escape(&c.to_string()));
                i += 1;
            }
        }
    }

    let start = if anchored { "^" } else { "(?:^|.*/)" };
    // What follows the match: nothing (the path itself) or anything under it
    // (a directory the pattern named). A directory-only pattern needs the
    // second, since a file is never a directory.
    let rest = if directory_only { "/.*$" } else { "(?:/.*)?$" };
    format!("{start}{out}{rest}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matches(pattern: &str, path: &str) -> bool {
        Patterns::new(&[pattern]).unwrap().matches(path)
    }

    #[test]
    fn a_bare_name_matches_at_any_depth() {
        assert!(matches("README.md", "README.md"));
        assert!(matches("README.md", "docs/README.md"));
        assert!(!matches("README.md", "README.md.bak"));
    }

    #[test]
    fn a_name_with_a_slash_is_anchored() {
        assert!(matches("src/main.rs", "src/main.rs"));
        assert!(!matches("src/main.rs", "crates/x/src/main.rs"));
        assert!(matches("/Cargo.lock", "Cargo.lock"));
        assert!(!matches("/Cargo.lock", "sub/Cargo.lock"));
    }

    #[test]
    fn a_directory_takes_everything_under_it() {
        assert!(matches("target", "target/debug/x"));
        assert!(matches("target/", "target/debug/x"));
        assert!(!matches("target/", "target"));
        assert!(matches("assets/", "web/assets/logo.png"));
    }

    #[test]
    fn stars_stay_within_a_directory_unless_doubled() {
        assert!(matches("*.png", "logo.png"));
        assert!(matches("*.png", "assets/logo.png"));
        assert!(!matches("assets/*.png", "assets/deep/logo.png"));
        assert!(matches("assets/**/*.png", "assets/deep/er/logo.png"));
        assert!(matches("assets/**/*.png", "assets/logo.png"));
        assert!(matches("?.txt", "a.txt"));
        assert!(!matches("?.txt", "ab.txt"));
    }

    #[test]
    fn dots_are_literal() {
        assert!(!matches("a.b", "axb"));
    }

    #[test]
    fn comments_and_blanks_are_skipped_and_negation_is_refused() {
        let patterns = Patterns::new(&["# a comment", "", "*.log"]).unwrap();
        assert!(patterns.matches("x.log"));
        assert!(Patterns::new(&["!keep.log"]).is_err());
    }
}
