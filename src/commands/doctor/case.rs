//! `case`: no two tracked paths differ only in case.
//!
//! Linux tells `Expedition.tsx` from `expedition.ts`; Windows and macOS keep
//! one name for both. An import of either resolves to whichever the file
//! system finds first, so the app that CI built green on Linux does not start
//! on the machine its author works on - and a checkout there writes one file
//! over the other.
//!
//! Two names need not be one path to clash. A module is imported without its
//! extension: `import './Expedition'` tries `.ts` before `.tsx`, and where the
//! file system cannot tell `Expedition` from `expedition` it finds
//! `expedition.ts` - the other module - beside `Expedition.tsx`. lyrid's web
//! UI did exactly this, so two modules of one directory whose names are one
//! but for case clash too.

use std::collections::BTreeMap;

use super::{Finding, Outcome, Repo, plural};

pub fn check(repo: &Repo) -> Outcome {
    let clashes = clashes(&repo.files);
    let modules = modules(&repo.files);
    if clashes.is_empty() && modules.is_empty() {
        return Outcome::Pass(format!(
            "{}, no two named alike but for case",
            plural(repo.files.len(), "tracked file", "tracked files")
        ));
    }
    let paths = clashes.into_iter().map(|names| {
        Finding::fail(format!("{} are one name on Windows and macOS", names.join(" and ")))
            .fix("rename one of them: an import of either reaches whichever the file system finds first")
    });
    let imports = modules.into_iter().map(|names| {
        let stem = names[0].rsplit('/').next().and_then(|n| n.rsplit_once('.')).map_or("", |(stem, _)| stem);
        Finding::fail(format!("{} are one module on Windows and macOS", names.join(" and "))).fix(format!(
            "rename one of them: `import './{stem}'` reaches whichever module the resolver tries first"
        ))
    });
    Outcome::Found(paths.chain(imports).collect())
}

/// What a module is imported without: the extensions a bundler or
/// TypeScript tries after a bare `./name`.
const RESOLVED: &[&str] = &["ts", "tsx", "js", "jsx", "mjs", "cjs", "mts", "cts"];

/// Every set of modules in one directory that are imported by one name but
/// for case - `Expedition.tsx` beside `expedition.ts` - with names that are
/// not already one path, which `clashes` says.
fn modules(files: &[String]) -> Vec<Vec<String>> {
    let mut by_module: BTreeMap<(String, String), Vec<&String>> = BTreeMap::new();
    for file in files {
        let (dir, name) = file.rsplit_once('/').unwrap_or(("", file));
        let Some((stem, extension)) = name.rsplit_once('.') else { continue };
        if stem.is_empty() || !RESOLVED.contains(&extension) {
            continue;
        }
        by_module.entry((dir.to_string(), stem.to_lowercase())).or_default().push(file);
    }
    let stem = |path: &str| {
        path.rsplit('/')
            .next()
            .and_then(|n| n.rsplit_once('.'))
            .map_or(String::new(), |(s, _)| s.to_string())
    };
    let mut found = Vec::new();
    for names in by_module.into_values() {
        // One spelling of the name is no clash, however many extensions it
        // has: `button.ts` beside `button.tsx` is a choice the resolver
        // makes the same way everywhere.
        let mut stems: Vec<String> = names.iter().map(|n| stem(n)).collect();
        stems.sort();
        stems.dedup();
        if stems.len() < 2 {
            continue;
        }
        // Names that are one path but for case are a clash of paths already.
        let mut lowered: Vec<String> = names.iter().map(|n| n.to_lowercase()).collect();
        lowered.sort();
        lowered.dedup();
        if lowered.len() < 2 {
            continue;
        }
        let mut names: Vec<String> = names.into_iter().cloned().collect();
        names.sort();
        found.push(names);
    }
    found
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

    /// lyrid's case: two files that are not one path, and are one module
    /// wherever case is not told apart.
    #[test]
    fn two_modules_named_alike_but_for_case_clash() {
        let found = modules(&files(&["web/src/Expedition.tsx", "web/src/expedition.ts", "web/src/main.tsx"]));
        assert_eq!(found, [vec!["web/src/Expedition.tsx".to_string(), "web/src/expedition.ts".to_string()]]);
        let repo_files = files(&["web/src/Expedition.tsx", "web/src/expedition.ts"]);
        let dir = tempfile::tempdir().unwrap();
        let mut repo = Repo::on_disk(dir.path());
        repo.files = repo_files;
        let outcome = check(&repo);
        let said: Vec<&str> = outcome.findings().iter().map(|f| f.what.as_str()).collect();
        assert_eq!(said, ["web/src/Expedition.tsx and web/src/expedition.ts are one module on Windows and macOS"]);
        assert!(outcome.findings()[0].fix.as_deref().unwrap().contains("`import './Expedition'`"));
    }

    #[test]
    fn modules_spelt_one_way_or_in_other_places_do_not_clash() {
        assert!(modules(&files(&["src/button.ts", "src/button.tsx", "src/Button.css", "src/button.test.ts"])).is_empty());
        assert!(modules(&files(&["src/a/Card.tsx", "src/b/card.ts", "README.md", "readme.ts"])).is_empty());
        // One path but for case is the clash of paths, said once there.
        assert!(modules(&files(&["src/Card.tsx", "src/card.tsx"])).is_empty());
    }

    #[test]
    fn names_that_differ_otherwise_do_not() {
        assert!(clashes(&files(&["README.md", "readme.txt", "src/main.rs", "Cargo.toml", "cargo.lock"])).is_empty());
    }
}
