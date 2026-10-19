//! `commands`: every command the README and the docs show exists, and every
//! command has a page.
//!
//! A README that shows a command the program does not have is worse than one
//! that shows nothing: it is confidently wrong, and it is the first thing a
//! reader copies. The other direction is the line's rule that a command
//! without a page does not exist. Both are read from the program itself -
//! `--help` of the build in `target/` - because a list kept beside the code
//! is one more thing to forget.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command as Process;

use regex::Regex;

use super::product::{Manifest, Product};
use super::site::Site;
use crate::commands::doctor::{Finding, Outcome, Repo, is_test_material, plural};

pub fn check(repo: &Repo, product: &Product) -> Outcome {
    let programs = product.programs();
    if programs.is_empty() {
        return Outcome::Skip("not a command-line program".to_string());
    }
    let mut findings = Vec::new();
    let mut shown_count = 0;
    let mut paged_count = 0;
    let mut site_note = String::new();
    for (manifest, bin) in programs {
        let program = match binary(repo, manifest, bin) {
            Ok(path) => path,
            Err(reason) => {
                findings.push(Finding::fail(format!("`{bin} --help` could not be read: {reason}")).fix(format!("`cargo build --bin {bin}`, then check again")));
                continue;
            }
        };
        let mut help = Help::new(&program);
        let top = match help.commands(&[]) {
            Ok(Some(commands)) => commands,
            Ok(None) => {
                findings.push(Finding::warn(format!("`{bin} --help` lists no commands")));
                continue;
            }
            Err(reason) => {
                findings.push(
                    Finding::fail(format!("`{bin} --help` failed: {reason}"))
                        .fix("parse the arguments before reading the configuration: `--help` answers anywhere, with no environment"),
                );
                continue;
            }
        };

        for (file, line, words) in shown(repo, bin) {
            shown_count += 1;
            let Some(first) = words.first() else { continue };
            if !top.iter().any(|c| c == first) {
                findings.push(
                    Finding::fail(format!("{file}:{line} shows `{bin} {first}`, which `{bin} --help` does not list"))
                        .fix("show a command the program has, or give the program the command"),
                );
                continue;
            }
            if let Some(second) = words.get(1)
                && let Ok(Some(inner)) = help.commands(&[first.as_str()])
                && !inner.iter().any(|c| c == second)
            {
                findings.push(
                    Finding::fail(format!(
                        "{file}:{line} shows `{bin} {first} {second}`, which `{bin} {first} --help` does not list"
                    ))
                    .fix("show a command the program has, or give the program the command"),
                );
            }
        }

        match Site::read(repo) {
            None => site_note = ", no documentation site to page them".to_string(),
            Some(site) => {
                let reference = site.reference_dir();
                let pages: Vec<&String> = repo.files.iter().filter(|f| f.starts_with(&format!("{reference}/"))).collect();
                if pages.is_empty() {
                    findings.push(
                        Finding::warn(format!("the documentation site has no pages under {reference}/"))
                            .fix("a page per command, named after it: the line's rule is that a command without a page does not exist"),
                    );
                    continue;
                }
                // A command named on any page of the site is documented: a
                // guide that walks through it is a page for it.
                let content = format!("{}/src/content/docs/", site.dir);
                let texts: Vec<String> = repo
                    .files
                    .iter()
                    .filter(|f| f.starts_with(&content) && (f.ends_with(".md") || f.ends_with(".mdx")))
                    .filter_map(|p| repo.read_text(p))
                    .collect();
                for command in help.names(&[]) {
                    let named = Regex::new(&format!(r"\b{}\s+{}\b", regex::escape(bin), regex::escape(&command))).expect("a valid pattern");
                    let own_page = pages.iter().any(|p| {
                        let rest = &p[reference.len() + 1..];
                        let stem = rest.split('/').next().unwrap_or(rest);
                        stem.strip_suffix(".md").or_else(|| stem.strip_suffix(".mdx")).unwrap_or(stem) == command
                    });
                    if own_page || texts.iter().any(|t| named.is_match(t)) {
                        paged_count += 1;
                    } else {
                        findings.push(
                            Finding::fail(format!("`{bin} {command}` has no page in {reference}/, and no page of the site names it"))
                                .fix(format!("write {reference}/{command}.md: a command without a page does not exist")),
                        );
                    }
                }
            }
        }
    }
    Outcome::from_findings(findings, || {
        let pages = if site_note.is_empty() {
            format!(", {} with a page", plural(paged_count, "command", "commands"))
        } else {
            site_note
        };
        format!("{} shown, all real{pages}", plural(shown_count, "command", "commands"))
    })
}

/// The commands a program's help lists, asked once per command path.
struct Help<'a> {
    program: &'a Path,
    asked: BTreeMap<Vec<String>, Result<Option<Listing>, String>>,
}

#[derive(Clone)]
struct Listing {
    /// Names and aliases, for what a page may show.
    accepted: Vec<String>,
    /// Names alone, for what needs a page.
    names: Vec<String>,
}

impl<'a> Help<'a> {
    fn new(program: &'a Path) -> Help<'a> {
        Help {
            program,
            asked: BTreeMap::new(),
        }
    }

    fn listing(&mut self, path: &[&str]) -> Result<Option<Listing>, String> {
        let key: Vec<String> = path.iter().map(|s| s.to_string()).collect();
        if let Some(found) = self.asked.get(&key) {
            return found.clone();
        }
        let found = Process::new(self.program)
            .args(path)
            .arg("--help")
            .output()
            .map_err(|e| e.to_string())
            .and_then(|out| {
                if out.status.success() {
                    Ok(parse_help(&String::from_utf8_lossy(&out.stdout)))
                } else {
                    Err(String::from_utf8_lossy(&out.stderr)
                        .lines()
                        .next()
                        .unwrap_or("it exited with an error")
                        .to_string())
                }
            });
        self.asked.insert(key, found.clone());
        found
    }

    fn commands(&mut self, path: &[&str]) -> Result<Option<Vec<String>>, String> {
        self.listing(path).map(|l| l.map(|l| l.accepted))
    }

    fn names(&mut self, path: &[&str]) -> Vec<String> {
        self.listing(path).ok().flatten().map(|l| l.names).unwrap_or_default()
    }
}

/// The commands under `Commands:` in clap's help, with their visible
/// aliases; `help` is clap's own and needs no page.
fn parse_help(help: &str) -> Option<Listing> {
    let mut lines = help.lines().skip_while(|l| l.trim() != "Commands:");
    lines.next()?;
    let alias = Regex::new(r"\[aliases?: ([^\]]+)\]").expect("a valid pattern");
    let mut listing = Listing {
        accepted: Vec::new(),
        names: Vec::new(),
    };
    for line in lines.take_while(|l| !l.trim().is_empty()) {
        // An entry is indented by two; a wrapped description, by more.
        let Some(entry) = line.strip_prefix("  ").filter(|e| !e.starts_with(char::is_whitespace)) else {
            continue;
        };
        let Some(name) = entry.split_whitespace().next() else { continue };
        if name == "help" {
            continue;
        }
        listing.accepted.push(name.to_string());
        listing.names.push(name.to_string());
        if let Some(captures) = alias.captures(entry) {
            listing.accepted.extend(captures[1].split(',').map(|a| a.trim().to_string()));
        }
    }
    Some(listing)
}

/// Every `$ <bin> <command> …` line in the README and the documentation, with
/// the words that name commands: up to the first flag or placeholder.
fn shown(repo: &Repo, bin: &str) -> Vec<(String, usize, Vec<String>)> {
    let prompt = format!("$ {bin} ");
    let mut found = Vec::new();
    for file in &repo.files {
        let documentation =
            file == "README.md" || ((file.starts_with("docs/") && !file.starts_with("docs/adr/")) && (file.ends_with(".md") || file.ends_with(".mdx")));
        if !documentation || is_test_material(file) {
            continue;
        }
        let Some(text) = repo.read_text(file) else { continue };
        for (i, line) in text.lines().enumerate() {
            let Some(rest) = line.trim_start().strip_prefix(&prompt) else { continue };
            let words: Vec<String> = rest
                .split_whitespace()
                .take_while(|w| !w.starts_with(['-', '<', '[', '{', '#', '"', '\'', '$', '|', '&', '>']))
                .take(2)
                .map(str::to_string)
                .collect();
            if !words.is_empty() {
                found.push((file.clone(), i + 1, words));
            }
        }
    }
    found
}

/// The program as built in `target/`: built again first if a source of the
/// package is newer, so the help read is the help of the code checked.
fn binary(repo: &Repo, manifest: &Manifest, bin: &str) -> Result<PathBuf, String> {
    let dir = repo.root.join(manifest.dir());
    let metadata = Process::new("cargo")
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .current_dir(&dir)
        .output()
        .map_err(|e| format!("cargo did not start: {e}"))?;
    if !metadata.status.success() {
        return Err(format!("`cargo metadata` failed: {}", String::from_utf8_lossy(&metadata.stderr).trim()));
    }
    let metadata: serde_json::Value = serde_json::from_slice(&metadata.stdout).map_err(|e| e.to_string())?;
    let target = metadata["target_directory"].as_str().ok_or("cargo did not say where it builds")?;
    let program = Path::new(target).join("debug").join(format!("{bin}{}", std::env::consts::EXE_SUFFIX));
    if !fresh(repo, &program) {
        let built = Process::new("cargo")
            // `--locked`: a check reads the repository and writes none of it,
            // and a build is otherwise free to rewrite Cargo.lock.
            .args(["build", "--quiet", "--locked", "--bin", bin])
            .current_dir(&dir)
            .output()
            .map_err(|e| format!("cargo did not start: {e}"))?;
        if !built.status.success() {
            let reason = String::from_utf8_lossy(&built.stderr);
            if reason.contains("--locked") {
                return Err("Cargo.lock disagrees with the manifests, and a check does not rewrite it - build once, then check again".to_string());
            }
            return Err(format!(
                "`cargo build --bin {bin}` failed: {}",
                reason.lines().find(|l| l.starts_with("error")).unwrap_or(reason.trim())
            ));
        }
    }
    if program.is_file() {
        Ok(program)
    } else {
        Err(format!("cargo built no {}", program.display()))
    }
}

/// Whether the program is newer than every source it is built from.
fn fresh(repo: &Repo, program: &Path) -> bool {
    let Ok(built) = std::fs::metadata(program).and_then(|m| m.modified()) else {
        return false;
    };
    repo.files
        .iter()
        .filter(|f| !is_test_material(f))
        .filter(|f| f.ends_with(".rs") || f.ends_with("Cargo.toml") || f.ends_with("Cargo.lock"))
        .filter_map(|f| std::fs::metadata(repo.root.join(f)).and_then(|m| m.modified()).ok())
        .all(|source| source <= built)
}

#[cfg(test)]
mod tests {
    use super::*;

    const HELP: &str = "Start a new web application\n\nUsage: lyrn.exe <COMMAND>\n\nCommands:\n  new       Create a new project in a directory\n            of its own\n  doctor    Hold a repository to the standard [aliases: dr]\n  help      Print this message or the help of the given subcommand(s)\n\nOptions:\n  -h, --help  Print help\n";

    #[test]
    fn the_commands_are_what_clap_lists() {
        let listing = parse_help(HELP).unwrap();
        assert_eq!(listing.names, ["new", "doctor"]);
        assert_eq!(listing.accepted, ["new", "doctor", "dr"]);
        assert!(parse_help("Usage: x [OPTIONS]\n\nOptions:\n  -h  Help\n").is_none());
    }
}
