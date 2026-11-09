//! `lyrn clean`: what a build or an install writes again, removed, and what
//! it weighed.
//!
//! Only a directory git ignores is ever a candidate, and of those only one a
//! tool of the stack writes again by itself: `target/`, `node_modules/`,
//! `dist/`, `.astro/`, `coverage/`. An ignored file is not one - a local
//! `.env` is ignored too, and nothing writes it again.

use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command as Process;

use crate::cli::CleanArgs;
use crate::rigger::Skipped;

/// The directories a tool of the stack writes again, and which tool.
pub const REBUILT: &[(&str, &str)] = &[
    ("target", "cargo builds it again"),
    ("node_modules", "the package manager installs it again"),
    ("dist", "the build writes it again"),
    (".astro", "the site's build writes it again"),
    ("coverage", "the tests write it again"),
];

/// One directory found to remove.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    /// `/`-separated, from the top of the repository, ending in `/`.
    pub path: String,
    pub bytes: u64,
    pub why: &'static str,
}

/// What became of one repository.
struct Cleaned {
    name: String,
    root: PathBuf,
    /// Why nothing was looked at, when nothing was.
    missing: Option<String>,
    found: Vec<Found>,
    /// What could not be removed, and why.
    refused: Vec<(String, String)>,
}

impl Cleaned {
    fn bytes(&self) -> u64 {
        self.found
            .iter()
            .filter(|f| !self.refused.iter().any(|(p, _)| *p == f.path))
            .map(|f| f.bytes)
            .sum()
    }
}

pub fn run(args: CleanArgs) -> Result<bool, Box<dyn Error>> {
    let (targets, skipped): (Vec<(String, PathBuf)>, Vec<Skipped>) = if args.all {
        crate::rigger::line()?
    } else {
        let paths = if args.paths.is_empty() {
            vec![PathBuf::from(".")]
        } else {
            args.paths.clone()
        };
        for path in &paths {
            if !path.is_dir() {
                return Err(format!("`{}` is not a directory - `lyrn clean` cleans a repository", path.display()).into());
            }
        }
        let named = paths
            .into_iter()
            .map(|path| {
                let name = std::fs::canonicalize(&path)
                    .ok()
                    .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                    .unwrap_or_else(|| path.display().to_string());
                (name, path)
            })
            .collect();
        (named, Vec::new())
    };

    let mut results = Vec::new();
    for (name, root) in &targets {
        results.push(clean(name, root, args.dry_run));
    }
    if results.len() == 1 && !args.all {
        print_one(&results[0], args.dry_run);
    } else {
        print_table(&results, &skipped, args.dry_run);
    }
    Ok(results.iter().all(|r| r.refused.is_empty() && (r.missing.is_none() || args.all)))
}

fn clean(name: &str, root: &Path, dry_run: bool) -> Cleaned {
    let mut cleaned = Cleaned {
        name: name.to_string(),
        root: root.to_path_buf(),
        missing: None,
        found: Vec::new(),
        refused: Vec::new(),
    };
    if !root.is_dir() {
        cleaned.missing = Some(format!("{} is not a directory", root.display()));
        return cleaned;
    }
    // The repository the directory is in, from its top: a directory of it
    // is cleaned as the whole of it, the way every command reads one.
    let top = Process::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(root)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| PathBuf::from(String::from_utf8_lossy(&o.stdout).trim()));
    if let Some(top) = top {
        cleaned.root = top;
    }
    let root = cleaned.root.clone();
    let root = root.as_path();
    match find(root) {
        Ok(found) => cleaned.found = found,
        Err(reason) => {
            cleaned.missing = Some(reason);
            return cleaned;
        }
    }
    if dry_run {
        return cleaned;
    }
    for found in &cleaned.found {
        let dir = root.join(found.path.trim_end_matches('/'));
        if let Err(error) = std::fs::remove_dir_all(&dir) {
            cleaned.refused.push((found.path.clone(), error.to_string()));
        }
    }
    cleaned
}

/// The directories to remove from the repository at `root`, each with what
/// it weighs, largest first.
pub fn find(root: &Path) -> Result<Vec<Found>, String> {
    // `--directory` names an ignored directory once, as `path/`, rather than
    // every file in it; one inside another ignored one is not named at all.
    let out = Process::new("git")
        .args(["ls-files", "--others", "--ignored", "--exclude-standard", "--directory", "-z"])
        .current_dir(root)
        .output()
        .map_err(|e| format!("git could not be run: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "not a git repository - `lyrn clean` removes only what git ignores: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    let listed = String::from_utf8_lossy(&out.stdout).into_owned();
    let mut found = Vec::new();
    for entry in listed.split('\0').filter(|e| e.ends_with('/')) {
        let last = entry.trim_end_matches('/').rsplit('/').next().unwrap_or_default();
        let Some((_, why)) = REBUILT.iter().find(|(name, _)| *name == last) else {
            continue;
        };
        let dir = root.join(entry.trim_end_matches('/'));
        // A link is somebody else's directory: removing through it would
        // empty the place it points at.
        if std::fs::symlink_metadata(&dir).map(|m| m.file_type().is_symlink()).unwrap_or(true) {
            continue;
        }
        found.push(Found {
            path: entry.to_string(),
            bytes: weight(&dir),
            why,
        });
    }
    found.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.path.cmp(&b.path)));
    Ok(found)
}

/// What a directory weighs on disk, links not followed.
fn weight(dir: &Path) -> u64 {
    let mut total = 0;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let Ok(meta) = entry.path().symlink_metadata() else { continue };
            if meta.file_type().is_symlink() {
                continue;
            }
            if meta.is_dir() {
                stack.push(entry.path());
            } else {
                total += meta.len();
            }
        }
    }
    total
}

/// A size the way a person reads it: `4.2 GB`, `312.0 MB`, `0 B`.
pub fn size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["KB", "MB", "GB", "TB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64 / 1024.0;
    let mut unit = 0;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}

fn print_one(cleaned: &Cleaned, dry_run: bool) {
    println!("lyrn clean: {}\n", cleaned.root.display());
    if let Some(reason) = &cleaned.missing {
        println!("✗ {reason}");
        return;
    }
    if cleaned.found.is_empty() {
        println!("Nothing to remove: no ignored directory here that a build or an install writes again.");
        return;
    }
    let width = cleaned.found.iter().map(|f| f.path.len()).max().unwrap_or(0);
    let sizes: Vec<String> = cleaned.found.iter().map(|f| size(f.bytes)).collect();
    let size_width = sizes.iter().map(String::len).max().unwrap_or(0);
    for (found, weighed) in cleaned.found.iter().zip(&sizes) {
        let refused = cleaned.refused.iter().find(|(p, _)| *p == found.path);
        let verb = match (dry_run, refused) {
            (true, _) => "would remove",
            (false, None) => "removed",
            (false, Some(_)) => "could not remove",
        };
        println!("{verb:<12}  {:<width$}  {weighed:>size_width$}  {}", found.path, found.why);
        if let Some((_, reason)) = refused {
            println!("              {reason}");
        }
    }
    let total = size(if dry_run {
        cleaned.found.iter().map(|f| f.bytes).sum()
    } else {
        cleaned.bytes()
    });
    if dry_run {
        println!("\n{total} to free: `lyrn clean` without --dry-run removes it.");
    } else {
        println!("\n{total} freed.");
    }
}

fn print_table(results: &[Cleaned], skipped: &[Skipped], dry_run: bool) {
    println!("lyrn clean: {} repositories\n", results.len());
    let width = results.iter().map(|r| r.name.len()).max().unwrap_or(0);
    let mut total = 0;
    let mut cleaned = 0;
    for result in results {
        if let Some(reason) = &result.missing {
            println!("  {:<width$}  - {reason}", result.name);
            continue;
        }
        if result.found.is_empty() {
            println!("  {:<width$}  nothing to remove", result.name);
            continue;
        }
        let bytes = if dry_run {
            result.found.iter().map(|f| f.bytes).sum()
        } else {
            result.bytes()
        };
        total += bytes;
        cleaned += 1;
        // The largest few: the rest of a row is the small change of a
        // repository, and `lyrn clean <path>` lists it all.
        const NAMED: usize = 3;
        let mut parts: Vec<String> = result.found.iter().take(NAMED).map(|f| format!("{} {}", f.path, size(f.bytes))).collect();
        if result.found.len() > NAMED {
            parts.push(format!("and {} more", result.found.len() - NAMED));
        }
        println!("  {:<width$}  {:>9}  {}", result.name, size(bytes), parts.join(", "));
        for (path, reason) in &result.refused {
            println!("  {:<width$}  ✗ {path} could not be removed: {reason}", "");
        }
    }
    for skip in skipped {
        println!("  {:<width$}  - {}", skip.name, skip.reason);
    }
    let verb = if dry_run { "to free" } else { "freed" };
    println!("\n{} {verb} in {cleaned} of {} repositories.", size(total), results.len());
    if dry_run && total > 0 {
        println!("Nothing was removed: `lyrn clean` without --dry-run removes it.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_size_is_read_in_its_unit() {
        assert_eq!(size(0), "0 B");
        assert_eq!(size(1023), "1023 B");
        assert_eq!(size(1024), "1.0 KB");
        assert_eq!(size(1536), "1.5 KB");
        assert_eq!(size(5 * 1024 * 1024), "5.0 MB");
        assert_eq!(size(3 * 1024 * 1024 * 1024 + 200 * 1024 * 1024), "3.2 GB");
    }

    fn git(dir: &Path, args: &[&str]) {
        let out = Process::new("git").args(args).current_dir(dir).output().unwrap();
        assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
    }

    /// What git ignores and a tool writes again is found; what git keeps, or
    /// ignores and nothing writes again, is not.
    #[test]
    fn only_ignored_directories_a_tool_writes_again_are_found() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        git(root, &["init", "--quiet"]);
        std::fs::write(root.join(".gitignore"), "target/\nnode_modules/\n.env\nsecrets/\ndocs/dist/\n").unwrap();
        for (path, bytes) in [
            ("target/debug/lyrn", 3000),
            ("docs/node_modules/astro/index.js", 200),
            ("docs/dist/index.html", 50),
            ("secrets/key", 10),
            (".env", 10),
            ("dist/kept.js", 5),
        ] {
            let file = root.join(path);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, vec![b'x'; bytes]).unwrap();
        }
        let found = find(root).unwrap();
        let paths: Vec<(&str, u64)> = found.iter().map(|f| (f.path.as_str(), f.bytes)).collect();
        assert_eq!(paths, [("target/", 3000), ("docs/node_modules/", 200), ("docs/dist/", 50)]);
        assert!(find(&root.join("..").join("no-such-directory-here")).is_err());
    }
}
