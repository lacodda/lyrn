//! `private`: nothing from this computer in the repository - its user's home,
//! the other projects beside it, its name.
//!
//! A path copied from a terminal carries the account it was copied on, and a
//! script that finds its files by an absolute path works on one machine only
//! and tells everyone else where its author keeps their projects. The line
//! found its own export scripts reading `<projects>/<name>/assets` by the
//! full path, and a test reading the vault's brand registry by the owner's
//! path - each worked, so nothing failed, and each was public.
//!
//! What counts is decided by this machine rather than guessed: its account's
//! home in any spelling, a path into the directory this repository lives in
//! that names something actually there, and the machine's own name. A made-up
//! example path is not this computer, and is left alone.

use std::path::{Component, Path, PathBuf};

use regex::Regex;

use super::{Finding, Outcome, Repo, plural, some_of};

/// One place a file names this computer.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Hit {
    file: String,
    line: usize,
}

pub fn check(repo: &Repo) -> Outcome {
    if repo.on_ci {
        return Outcome::Skip("a CI runner's home and name are not yours".to_string());
    }
    let machine = Machine::this();
    let neighbourhood = repo.root.parent().filter(|p| is_a_place_of_projects(p)).map(Path::to_path_buf);

    let mut home_hits = Vec::new();
    let mut place_hits: Vec<(Hit, String)> = Vec::new();
    let mut name_hits = Vec::new();
    let home = machine.account.as_deref().map(home_pattern);
    let place = neighbourhood.as_deref().and_then(path_pattern);
    let name = machine
        .name
        .as_deref()
        .map(|n| Regex::new(&format!(r"(?i)\b{}\b", regex::escape(n))).expect("an escaped name"));

    // Each pattern runs over a whole file at once; a match is rare, and only
    // then is its line counted.
    for (file, text) in repo.texts() {
        let hit = |at: usize| Hit {
            file: file.clone(),
            line: text[..at].matches('\n').count() + 1,
        };
        if let Some(pattern) = &home {
            home_hits.extend(pattern.find_iter(text).map(|m| hit(m.start())));
        }
        if let (Some(pattern), Some(dir)) = (&place, &neighbourhood) {
            for captures in pattern.captures_iter(text) {
                let entry = captures[1].trim_end_matches('.');
                if !entry.is_empty() && dir.join(entry).exists() {
                    place_hits.push((hit(captures.get(0).map_or(0, |m| m.start())), entry.to_string()));
                }
            }
        }
        if let Some(pattern) = &name {
            name_hits.extend(pattern.find_iter(text).map(|m| hit(m.start())));
        }
    }

    let mut findings = Vec::new();
    if !home_hits.is_empty() {
        let account = machine.account.as_deref().unwrap_or_default();
        findings.push(
            Finding::fail(format!("the home of `{account}`, this computer's account, in {}", places(&home_hits)))
                .fix("a path from the repository's top, or `~` and `$HOME` where a home is meant; in an example, a made-up name"),
        );
    }
    if !place_hits.is_empty() {
        let dir = neighbourhood.as_deref().map(|d| d.display().to_string()).unwrap_or_default();
        let mut named: Vec<String> = place_hits.iter().map(|(_, entry)| entry.clone()).collect();
        named.dedup();
        let hits: Vec<Hit> = place_hits.into_iter().map(|(hit, _)| hit).collect();
        findings.push(
            Finding::fail(format!(
                "a path into {dir}, where this repository lives, naming {} - in {}",
                some_of(&named, 3),
                places(&hits)
            ))
            .fix("a path from the repository's top - resolved from the script's own location - works on every machine"),
        );
    }
    if !name_hits.is_empty() {
        let name = machine.name.as_deref().unwrap_or_default();
        findings.push(Finding::fail(format!("`{name}`, this computer's name, in {}", places(&name_hits))).fix("leave it out, or use a made-up host"));
    }

    Outcome::from_findings(findings, || {
        format!("nothing from this computer in {}", plural(repo.texts().len(), "text file", "text files"))
    })
}

/// `docs/a.mjs:8` for the first few, and how many more.
fn places(hits: &[Hit]) -> String {
    let named: Vec<String> = hits.iter().map(|h| format!("{}:{}", h.file, h.line)).collect();
    some_of(&named, 3)
}

/// What this machine is called, and whose it is.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Machine {
    /// The account whose home this is: the last part of the home directory.
    account: Option<String>,
    name: Option<String>,
}

impl Machine {
    fn this() -> Machine {
        let home = std::env::var_os("USERPROFILE")
            .filter(|_| cfg!(windows))
            .or_else(|| std::env::var_os("HOME"))
            .map(PathBuf::from);
        let account = home
            .as_deref()
            .and_then(Path::file_name)
            .map(|n| n.to_string_lossy().into_owned())
            .filter(|n| n.len() >= 2);
        let name = std::env::var("COMPUTERNAME")
            .ok()
            .or_else(|| std::env::var("HOSTNAME").ok())
            .or_else(|| std::fs::read_to_string("/etc/hostname").ok())
            .map(|n| n.trim().to_string())
            // A short or generic name is a word, not this machine.
            .filter(|n| n.len() >= 4 && !n.eq_ignore_ascii_case("localhost"));
        Machine { account, name }
    }
}

/// The account's home in every spelling a file meets it in: `C:\Users\ada`,
/// `C:\\Users\\ada` inside a string, `/c/Users/ada` from Git Bash,
/// `/home/ada`, `/Users/ada`.
fn home_pattern(account: &str) -> Regex {
    Regex::new(&format!(
        r"(?i)(?:^|[^A-Za-z0-9])(?:users|home)(?:\\|/)+{}(?:$|[^A-Za-z0-9_-])",
        regex::escape(account)
    ))
    .expect("an escaped account")
}

/// A path into `dir`, in any of its spellings, with the name of what it
/// points at captured.
fn path_pattern(dir: &Path) -> Option<Regex> {
    let mut parts: Vec<String> = Vec::new();
    let mut drive: Option<String> = None;
    for component in dir.components() {
        match component {
            Component::Prefix(prefix) => {
                let text = prefix.as_os_str().to_string_lossy().into_owned();
                drive = Some(text.trim_end_matches(':').to_string());
            }
            Component::Normal(part) => parts.push(regex::escape(&part.to_string_lossy())),
            _ => {}
        }
    }
    if parts.is_empty() {
        return None;
    }
    let separator = r"(?:\\|/)+";
    let start = match &drive {
        // `C:\x`, `C:/x`, `/c/x` from Git Bash, `/mnt/c/x` from WSL.
        Some(letter) => format!(r"(?:\b{0}:|/(?:mnt/)?{0}){separator}", regex::escape(letter)),
        None => separator.to_string(),
    };
    let case = if cfg!(windows) { "(?i)" } else { "" };
    let pattern = format!(r"{case}{start}{}{separator}([A-Za-z0-9._-]+)", parts.join(separator));
    Regex::new(&pattern).ok()
}

/// Whether a directory is a place someone keeps projects, rather than one
/// every machine has: the system's temporary directory and the top of a disk
/// say nothing about anyone.
fn is_a_place_of_projects(dir: &Path) -> bool {
    let normal = dir.components().filter(|c| matches!(c, Component::Normal(_))).count();
    if normal == 0 {
        return false;
    }
    let temp = std::env::temp_dir();
    let same = |a: &Path, b: &Path| a.canonicalize().ok().zip(b.canonicalize().ok()).is_some_and(|(a, b)| a == b);
    !same(dir, &temp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_home_is_found_in_every_spelling() {
        let home = home_pattern("ada");
        for line in [
            r"C:\Users\ada\notes.txt",
            r#""C:\\Users\\ada\\x""#,
            "/c/Users/ada/src",
            "cd /home/ada",
            "/Users/Ada/Library",
        ] {
            assert!(home.is_match(line), "{line}");
        }
        for line in [r"C:\Users\adam\x", "/home/adalovelace", "the users of ada"] {
            assert!(!home.is_match(line), "{line}");
        }
    }

    #[test]
    fn a_path_into_the_place_of_projects_names_what_it_points_at() {
        let dir = if cfg!(windows) {
            PathBuf::from(r"D:\work")
        } else {
            PathBuf::from("/srv/work")
        };
        let pattern = path_pattern(&dir).unwrap();
        let lines: &[&str] = if cfg!(windows) {
            &[
                r#"const ASSETS = "D:/work/demo/assets""#,
                r"see d:\work\demo\src",
                "/d/work/demo",
                r#""D:\\work\\demo""#,
            ]
        } else {
            &["const ASSETS = \"/srv/work/demo/assets\""]
        };
        for line in lines {
            let captures = pattern.captures(line).unwrap_or_else(|| panic!("{line}"));
            assert_eq!(&captures[1], "demo", "{line}");
        }
        let elsewhere = if cfg!(windows) { "D:/workshop/demo" } else { "/srv/workshop/demo" };
        assert!(!pattern.is_match(elsewhere), "a directory whose name starts the same is another place");
    }

    #[test]
    fn the_top_of_a_disk_and_the_temporary_directory_are_nobody_s() {
        assert!(!is_a_place_of_projects(&std::env::temp_dir()));
        let root = if cfg!(windows) { PathBuf::from(r"C:\") } else { PathBuf::from("/") };
        assert!(!is_a_place_of_projects(&root));
    }
}
