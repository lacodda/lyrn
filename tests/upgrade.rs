//! `lyrn upgrade`, run on repositories generated for the purpose and then
//! set back to the oldest standard the archive keeps - the way a repository
//! adopted with lyrn 2.10 stands today.
//!
//! The merge itself is held file by file, for every form, in the command's
//! unit tests; these hold what only the binary does: the commit, the refusal
//! to touch uncommitted work, the dry run, and the exit code of a run that
//! left a place for a person.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

mod common;

const OLDEST: &str = "2026.10";

/// The standard the templates write now, read from them.
fn current() -> String {
    let manifest: toml::Table = toml::from_str(include_str!("../src/templates/cli/template.toml")).unwrap();
    manifest["standard"].as_str().unwrap().to_string()
}

/// The versions the archive keeps after the oldest, in order: the ones an
/// upgrade from it walks through.
fn versions_after_the_oldest() -> Vec<String> {
    let archive = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/standard/archive");
    let parts = |v: &str| v.split('.').map(|p| p.parse::<u32>().unwrap()).collect::<Vec<_>>();
    let mut versions: Vec<String> = fs::read_dir(archive)
        .unwrap()
        .filter_map(|e| e.unwrap().file_name().to_string_lossy().strip_suffix(".toml").map(str::to_string))
        .filter(|v| v != OLDEST)
        .collect();
    versions.sort_by_key(|v| parts(v));
    versions
}

struct Machine {
    _dir: tempfile::TempDir,
    global: PathBuf,
}

impl Machine {
    fn new() -> Machine {
        let dir = tempfile::tempdir().unwrap();
        let global = dir.path().join(".gitconfig");
        fs::write(
            &global,
            "[user]\n\tname = Ada Lovelace\n\temail = ada@example.com\n[init]\n\tdefaultBranch = main\n[commit]\n\tgpgsign = false\n",
        )
        .unwrap();
        Machine { _dir: dir, global }
    }

    fn git(&self, root: &Path, args: &[&str]) -> String {
        let out = Command::new("git")
            .args(args)
            .current_dir(root)
            .env("GIT_CONFIG_GLOBAL", &self.global)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .output()
            .unwrap();
        assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    fn upgrade(&self, root: &Path, args: &[&str]) -> (String, bool) {
        let out = common::lyrn()
            .arg("upgrade")
            .args(args)
            .arg(root)
            .env("GIT_CONFIG_GLOBAL", &self.global)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .output()
            .unwrap();
        let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
        (text, out.status.success())
    }

    /// A cli project as this lyrn writes it, committed, then set back to the
    /// oldest standard: lyrn.toml says it, and the shopfront workflow - which
    /// that standard did not have - is gone.
    fn project_on_the_oldest(&self, dir: &Path) -> PathBuf {
        let root = dir.join("demo-app");
        common::lyrn()
            .args([
                "new",
                "demo-app",
                "--form",
                "cli",
                "--yes",
                "--no-hooks",
                "--accent",
                "#6D7BF2",
                "--repo",
                "acme/demo-app",
                "--path",
            ])
            .arg(&root)
            .assert()
            .success();
        // lyrn.toml as that standard wrote it: its version, and none of the
        // lines recording the add-ons, which came later.
        let stamp = fs::read_to_string(root.join("lyrn.toml")).unwrap();
        let older: String = stamp
            .replace(&format!("standard = \"{}\"", current()), &format!("standard = \"{OLDEST}\""))
            .lines()
            .take_while(|l| !l.starts_with("# The add-ons it was generated with"))
            .map(|l| format!("{l}\n"))
            .collect();
        fs::write(root.join("lyrn.toml"), older).unwrap();
        fs::remove_file(root.join(".github/workflows/shopfronts.yml")).unwrap();
        self.git(&root, &["init", "--quiet"]);
        self.git(&root, &["add", "-A"]);
        self.git(&root, &["commit", "--quiet", "--no-verify", "-m", "feat: start"]);
        root
    }
}

#[test]
fn an_older_standard_is_caught_up_with_in_one_commit() {
    let machine = Machine::new();
    let dir = tempfile::tempdir().unwrap();
    let root = machine.project_on_the_oldest(dir.path());
    let to = current();

    let (report, passed) = machine.upgrade(&root, &["--yes"]);
    assert!(passed, "{report}");
    // Every version on the way, in order, each named on a line of its own.
    let named: Vec<&str> = report
        .split_once("What the standard changed since:\n")
        .map(|(_, rest)| {
            rest.lines()
                .take_while(|l| !l.is_empty())
                .filter(|l| l.starts_with("  ") && !l.starts_with("   "))
                .map(str::trim)
                .collect()
        })
        .unwrap_or_default();
    assert_eq!(
        named,
        versions_after_the_oldest(),
        "not every version from {OLDEST} to {to}, in order, in:\n{report}"
    );
    for line in [
        format!("from the standard {OLDEST} to {to}"),
        "added".to_string(),
        format!("Committed: chore(standard): take the line's standard {to}"),
    ] {
        assert!(report.contains(&line), "no `{line}` in:\n{report}");
    }
    assert!(
        report.lines().any(|l| l.contains(".github/workflows/shopfronts.yml") && l.contains("added")),
        "{report}"
    );
    assert!(report.lines().any(|l| l.contains("lyrn.toml") && l.contains("updated")), "{report}");

    // The commit is the changelog's line: its subject, and what changed below.
    assert_eq!(
        machine.git(&root, &["log", "-1", "--format=%s"]).trim(),
        format!("chore(standard): take the line's standard {to}")
    );
    let body = machine.git(&root, &["log", "-1", "--format=%b"]);
    assert!(body.contains(&format!("Brought from the standard {OLDEST} by lyrn upgrade.")), "{body}");
    assert!(body.contains("shopfronts.yml"), "{body}");
    assert_eq!(machine.git(&root, &["status", "--porcelain"]), "", "the upgrade left something uncommitted");

    assert!(fs::read_to_string(root.join("lyrn.toml")).unwrap().contains(&format!("standard = \"{to}\"")));
    let workflow = fs::read_to_string(root.join(".github/workflows/shopfronts.yml")).unwrap();
    assert!(workflow.contains(&format!("LYRN_VERSION=v{}", env!("CARGO_PKG_VERSION"))), "{workflow}");

    // Caught up is caught up.
    let (report, passed) = machine.upgrade(&root, &["--yes"]);
    assert!(
        passed && report.contains(&format!("is on the standard {to}, the newest this lyrn writes: nothing to do")),
        "{report}"
    );
}

#[test]
fn a_dry_run_shows_the_diffs_and_writes_nothing() {
    let machine = Machine::new();
    let dir = tempfile::tempdir().unwrap();
    let root = machine.project_on_the_oldest(dir.path());
    let head = machine.git(&root, &["rev-parse", "HEAD"]);

    let (report, passed) = machine.upgrade(&root, &["--dry-run"]);
    assert!(passed, "{report}");
    assert!(report.contains("+++ b/.github/workflows/shopfronts.yml"), "{report}");
    assert!(report.contains(&format!("+standard = \"{}\"", current())), "{report}");
    assert!(report.trim_end().ends_with("Nothing was written."), "{report}");
    assert!(!root.join(".github/workflows/shopfronts.yml").exists());
    assert_eq!(machine.git(&root, &["rev-parse", "HEAD"]), head);
    assert_eq!(machine.git(&root, &["status", "--porcelain"]), "");
}

/// The merge reads the file as the repository's version, and the commit
/// would carry the person's change under the standard's name: work not yet
/// committed stops it before anything is written.
#[test]
fn uncommitted_work_in_its_files_stops_it() {
    let machine = Machine::new();
    let dir = tempfile::tempdir().unwrap();
    let root = machine.project_on_the_oldest(dir.path());
    let stamp = fs::read_to_string(root.join("lyrn.toml")).unwrap();
    fs::write(root.join("lyrn.toml"), format!("{stamp}# a note not committed yet\n")).unwrap();

    let (report, passed) = machine.upgrade(&root, &["--yes"]);
    assert!(!passed, "{report}");
    assert!(
        report.contains("nothing was written: the upgrade changes files with changes of your own not yet committed"),
        "{report}"
    );
    assert!(report.contains("lyrn.toml"), "{report}");
    assert!(!root.join(".github/workflows/shopfronts.yml").exists());
}

/// Where the repository changed the line the standard changes, the file is
/// marked, nothing is committed, and the run ends non-zero.
#[test]
fn a_place_both_changed_is_left_marked_and_uncommitted() {
    let machine = Machine::new();
    let dir = tempfile::tempdir().unwrap();
    let root = machine.project_on_the_oldest(dir.path());
    let stamp = fs::read_to_string(root.join("lyrn.toml")).unwrap();
    let held = format!("standard = \"{OLDEST}\" # held here on purpose");
    fs::write(root.join("lyrn.toml"), stamp.replace(&format!("standard = \"{OLDEST}\""), &held)).unwrap();
    machine.git(&root, &["commit", "--quiet", "--no-verify", "-am", "chore: hold the standard"]);
    let head = machine.git(&root, &["rev-parse", "HEAD"]);

    let (report, passed) = machine.upgrade(&root, &["--yes"]);
    assert!(!passed, "a run that left a place for a hand passed:\n{report}");
    assert!(report.contains("1 file needs a hand"), "{report}");
    assert!(
        report.contains(&format!("git commit -m \"chore(standard): take the line's standard {}\"", current())),
        "{report}"
    );
    let marked = fs::read_to_string(root.join("lyrn.toml")).unwrap();
    for line in ["<<<<<<< yours", held.as_str(), &format!(">>>>>>> standard {}", current())] {
        assert!(marked.lines().any(|l| l == line), "no `{line}` in:\n{marked}");
    }
    assert_eq!(machine.git(&root, &["rev-parse", "HEAD"]), head, "a marked upgrade was committed");

    // The doctor sees the place too, until it is resolved.
    let out = common::lyrn()
        .args(["doctor"])
        .arg(&root)
        .env("GIT_CONFIG_GLOBAL", &machine.global)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    let doctor = String::from_utf8_lossy(&out.stdout);
    assert!(doctor.contains("1 file keeps a place `lyrn upgrade` left for a hand: lyrn.toml"), "{doctor}");
}

#[test]
fn a_repository_without_lyrn_toml_is_sent_to_adopt() {
    let machine = Machine::new();
    let dir = tempfile::tempdir().unwrap();
    let (report, passed) = machine.upgrade(dir.path(), &["--yes"]);
    assert!(!passed);
    assert!(
        report.contains("has no lyrn.toml, so nothing says which standard it is on - `lyrn adopt` makes it a lyrn project first"),
        "{report}"
    );
}
