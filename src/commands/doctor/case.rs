//! `case`: no two tracked paths differ only in case.
//!
//! Linux tells `Expedition.tsx` from `expedition.ts`; Windows and macOS keep
//! one name for both. An import of either resolves to whichever the file
//! system finds first, so the app that CI built green on Linux does not start
//! on the machine its author works on - and a checkout there writes one file
//! over the other.

use std::collections::BTreeMap;

use super::{Finding, Outcome, Repo, plural};

pub fn check(repo: &Repo) -> Outcome {
    let clashes = clashes(&repo.files);
    if clashes.is_empty() {
        return Outcome::Pass(format!(
            "{}, no two named alike but for case",
            plural(repo.files.len(), "tracked file", "tracked files")
        ));
    }
    Outcome::Found(
        clashes
            .into_iter()
            .map(|names| {
                Finding::fail(format!("{} are one name on Windows and macOS", names.join(" and ")))
                    .fix("rename one of them: an import of either reaches whichever the file system finds first")
            })
            .collect(),
    )
}

/// Every set of spellings of one path, a directory's or a file's, that differ
/// only in case - each set once, at the shortest path it starts at.
fn clashes(files: &[String]) -> Vec<Vec<String>> {
    let mut spellings: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for file in files {
        let mut prefix = String::new();
        for part in file.split('/') {
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(part);
            let seen = spellings.entry(prefix.to_lowercase()).or_default();
            if !seen.contains(&prefix) {
                seen.push(prefix.clone());
            }
        }
    }
    let mut found: Vec<Vec<String>> = Vec::new();
    for (lower, mut names) in spellings {
        if names.len() < 2 {
            continue;
        }
        // A clash of directories is said once, not again for every file in
        // them.
        if found
            .iter()
            .any(|f| f.first().is_some_and(|n| lower.starts_with(&format!("{}/", n.to_lowercase()))))
        {
            continue;
        }
        names.sort();
        found.push(names);
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files(paths: &[&str]) -> Vec<String> {
        paths.iter().map(|p| p.to_string()).collect()
    }

    #[test]
    fn two_files_named_alike_but_for_case_clash() {
        let found = clashes(&files(&["web/src/Expedition.tsx", "web/src/expedition.ts", "web/src/expedition.tsx"]));
        assert_eq!(found, [vec!["web/src/Expedition.tsx".to_string(), "web/src/expedition.tsx".to_string()]]);
    }

    #[test]
    fn two_directories_named_alike_clash_once() {
        let found = clashes(&files(&["src/Views/a.rs", "src/views/b.rs", "src/views/c.rs"]));
        assert_eq!(found, [vec!["src/Views".to_string(), "src/views".to_string()]]);
    }

    #[test]
    fn names_that_differ_otherwise_do_not() {
        assert!(clashes(&files(&["README.md", "readme.txt", "src/main.rs", "Cargo.toml", "cargo.lock"])).is_empty());
    }
}
