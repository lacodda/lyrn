//! `lyrn doctor`, run the way a person runs it: on repositories made for the
//! purpose, on a machine whose git config is a file the test owns.
//!
//! Two promises are held from outside. Every form lyrn writes passes its own
//! doctor - otherwise the standard would contradict itself on the day a
//! project starts. And a repository that repeats each lesson the line learnt
//! fails each rule, by the finding that names it.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

mod common;

/// A machine: a global git config and a home the test owns, so neither the
/// developer's identity nor their excludes file leaks into what is measured.
struct Machine {
    _dir: tempfile::TempDir,
    home: PathBuf,
    global: PathBuf,
}

const OWNER_NAME: &str = "Ada Lovelace";
const OWNER_EMAIL: &str = "ada@example.com";

impl Machine {
    /// A machine with the owner's identity and an excludes file that keeps
    /// every assistant file out.
    fn new() -> Machine {
        let machine = Machine::bare();
        let excludes = machine.home.join("excludes");
        fs::write(&excludes, include_str!("../src/templates/community/gitignore.tmpl")).unwrap();
        fs::write(
            &machine.global,
            format!(
                "[user]\n\tname = {OWNER_NAME}\n\temail = {OWNER_EMAIL}\n[core]\n\texcludesFile = {}\n[init]\n\tdefaultBranch = main\n",
                excludes.display().to_string().replace('\\', "/")
            ),
        )
        .unwrap();
        machine
    }

    /// A machine with nothing configured.
    fn bare() -> Machine {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("home");
        fs::create_dir_all(&home).unwrap();
        let global = home.join(".gitconfig");
        fs::write(&global, "").unwrap();
        Machine { _dir: dir, home, global }
    }

    fn env(&self, command: &mut Command) {
        command
            .env("GIT_CONFIG_GLOBAL", &self.global)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("HOME", &self.home)
            .env("XDG_CONFIG_HOME", self.home.join(".config"))
            .env_remove("CI")
            .env_remove("GIT_AUTHOR_NAME")
            .env_remove("GIT_AUTHOR_EMAIL")
            .env_remove("GIT_COMMITTER_NAME")
            .env_remove("GIT_COMMITTER_EMAIL");
    }

    fn git(&self, repo: &Path, args: &[&str]) {
        let mut command = Command::new("git");
        command.args(args).current_dir(repo);
        self.env(&mut command);
        let out = command.output().unwrap();
        assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
    }

    /// A commit of everything, as whoever `who` is - the machine's identity
    /// when `None`.
    fn commit(&self, repo: &Path, message: &str, who: Option<(&str, &str)>) {
        self.git(repo, &["add", "-A"]);
        let mut args = Vec::new();
        let config;
        if let Some((name, email)) = who {
            config = [format!("user.name={name}"), format!("user.email={email}")];
            args.extend(["-c", config[0].as_str(), "-c", config[1].as_str()]);
        }
        args.extend(["commit", "--quiet", "--no-verify", "--allow-empty", "-m", message]);
        self.git(repo, &args);
    }

    fn lyrn(&self) -> assert_cmd::Command {
        let mut cmd = common::lyrn();
        cmd.env("GIT_CONFIG_GLOBAL", &self.global)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("HOME", &self.home)
            .env("XDG_CONFIG_HOME", self.home.join(".config"))
            .env_remove("CI");
        cmd
    }

    /// `lyrn doctor <repo>`: its report and whether it passed.
    fn doctor(&self, repo: &Path) -> (String, bool) {
        let out = self.lyrn().arg("doctor").arg(repo).output().unwrap();
        (String::from_utf8(out.stdout).unwrap(), out.status.success())
    }
}

/// The lines of a report that are findings, of either level.
fn findings(report: &str) -> Vec<&str> {
    report
        .lines()
        .filter(|l| l.starts_with("✗ ") || l.starts_with("! ") || l.starts_with("    ✗") || l.starts_with("    !"))
        .collect()
}

fn write(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// Where each form's lockfiles land once its dependencies are installed. The
/// generation runs without hooks, so the files are stood in for: the doctor
/// reads which lockfiles exist, not what they lock.
fn lockfiles(form: &str, with: &[&str]) -> Vec<&'static str> {
    let mut files = match form {
        "spa" | "mono" => vec!["pnpm-lock.yaml"],
        "desktop" => vec!["pnpm-lock.yaml", "src-tauri/Cargo.lock"],
        "tauri-plugin" => vec!["pnpm-lock.yaml", "Cargo.lock"],
        _ => vec!["Cargo.lock"],
    };
    if form == "service" && with.contains(&"spa") {
        files.push("frontend/pnpm-lock.yaml");
    }
    files
}

#[test]
fn every_form_lyrn_writes_passes_its_own_doctor() {
    let machine = Machine::new();
    let cases: &[(&str, &[&str])] = &[
        ("spa", &["router", "tanstack-query", "auth", "pwa"]),
        ("cli", &["keyring", "self-update"]),
        ("desktop", &["i18n"]),
        ("service", &["spa", "demo"]),
        ("workspace", &["keyring", "self-update"]),
        ("mono", &["stand"]),
        ("plugin", &[]),
        ("tauri-plugin", &[]),
    ];
    for (form, with) in cases {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("demo-app");
        let mut cmd = machine.lyrn();
        cmd.args([
            "new",
            "demo-app",
            "--form",
            form,
            "--yes",
            "--no-hooks",
            "--accent",
            "#6D7BF2",
            "--repo",
            "owner/demo-app",
            "--path",
        ])
        .arg(&root);
        if *form == "plugin" {
            cmd.args(["--host", "kilna"]);
        }
        if !with.is_empty() {
            cmd.args(["--with", &with.join(",")]);
        }
        cmd.assert().success();
        for lockfile in lockfiles(form, with) {
            write(&root, lockfile, "");
        }
        machine.git(&root, &["init", "--quiet"]);
        machine.commit(&root, "feat: start", None);

        let (report, passed) = machine.doctor(&root);
        assert!(passed && findings(&report).is_empty(), "{form} fails its own doctor:\n{report}");
        // The rules that read what the form wrote have something to read.
        assert!(report.contains("✓ standard"), "{form}:\n{report}");
        assert!(report.contains("✓ audit"), "{form}:\n{report}");
        assert!(report.contains("✓ actions"), "{form}:\n{report}");
    }
}

/// The docs form adds a site with its own lockfile, and its own audit for it -
/// the 2.10.1 lesson, held from the outside.
#[test]
fn a_documentation_site_lyrn_adds_is_audited() {
    let machine = Machine::new();
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("demo-tool");
    machine
        .lyrn()
        .args([
            "new",
            "demo-tool",
            "--form",
            "cli",
            "--yes",
            "--no-hooks",
            "--accent",
            "#6D7BF2",
            "--repo",
            "owner/demo-tool",
            "--path",
        ])
        .arg(&root)
        .assert()
        .success();
    machine
        .lyrn()
        .current_dir(&root)
        .args(["new", "demo-tool", "--form", "docs", "--yes", "--no-hooks", "--repo", "owner/demo-tool"])
        .assert()
        .success();
    write(&root, "Cargo.lock", "");
    write(&root, "docs/pnpm-lock.yaml", "");
    machine.git(&root, &["init", "--quiet"]);
    machine.commit(&root, "feat: start", None);

    let (report, passed) = machine.doctor(&root);
    assert!(passed && findings(&report).is_empty(), "{report}");
    assert!(report.contains("✓ audit      2 lockfiles audited"), "{report}");
}

/// One repository that repeats every lesson: each rule fails, by the finding
/// that names its lesson.
#[test]
fn a_repository_with_the_lessons_unlearnt_fails_each_rule() {
    let machine = Machine::new();
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(root, "README.md", "# demo\n\n> A demo.\n\n## Roadmap\n\nLater.\n");
    write(root, "Cargo.toml", "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2024\"\n");
    write(root, "src/main.rs", "fn main() {}\n");
    write(
        root,
        ".github/workflows/ci.yml",
        "name: CI\non: [push]\njobs:\n  build:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v7\n      - uses: actions/setup-node@v7\n        with:\n          node-version: 18\n      - run: echo \"::set-output name=x::y\"\n      - uses: actions/upload-artifact@v3\n      - uses: actions-rs/toolchain@v1\n",
    );
    write(root, "docs/package.json", "{ \"name\": \"demo-docs\", \"private\": true }\n");
    write(root, "docs/pnpm-lock.yaml", "");
    write(root, "CLAUDE.md", "notes for an assistant\n");
    machine.git(root, &["init", "--quiet"]);
    // The machine's excludes file would keep CLAUDE.md out; it is added the
    // way it reaches a repository on a machine without one.
    machine.git(root, &["add", "-f", "CLAUDE.md"]);
    machine.commit(root, "feat: start", None);
    write(root, "src/lib.rs", "\n");
    machine.commit(
        root,
        "feat: more\n\nCo-Authored-By: Claude <noreply@anthropic.com>",
        Some(("lacodda", "old@example.com")),
    );

    let (report, passed) = machine.doctor(root);
    assert!(!passed, "the doctor passed a repository that fails every rule:\n{report}");
    for expected in [
        "✗ standard",
        "not a lyrn project yet - no lyrn.toml",
        "✗ readme",
        "a \"Roadmap\" section",
        "- mark       no lyrn.toml to say",
        "✗ actions",
        "actions/upload-artifact@v3 in ci.yml: v1-v3 stopped working",
        "actions-rs/toolchain@v1 in ci.yml",
        "`::set-output` in ci.yml",
        "✗ msrv",
        "Cargo.toml declares no rust-version",
        "✗ node",
        "Node 18 in ci.yml",
        "docs/package.json names no packageManager",
        "✗ audit",
        "docs/pnpm-lock.yaml: no workflow audits it",
        "✗ ai-traces",
        ".gitignore keeps assistant files out",
        "tracked: CLAUDE.md",
        "1 commit carries an assistant's stamp",
        "✗ authors",
        "`lacodda <old@example.com>` wrote 1 commit and is neither you nor anyone .mailmap names",
        "`Ada Lovelace <ada@example.com> lacodda <old@example.com>`",
        "✓ identity",
        "why: lyrn doctor --explain authors",
    ] {
        assert!(report.contains(expected), "the report does not say `{expected}`:\n{report}");
    }
    // Only Cargo.lock that exists is reported: none was written here.
    assert!(!report.contains("Cargo.lock: no workflow"), "{report}");

    // The alias mapped, the history is one person again - without a rewrite.
    write(root, ".mailmap", &format!("{OWNER_NAME} <{OWNER_EMAIL}> lacodda <old@example.com>\n"));
    machine.commit(root, "chore: say who is who", None);
    let (report, _) = machine.doctor(root);
    assert!(report.contains("✓ authors    3 commits, all by Ada Lovelace <ada@example.com>"), "{report}");
}

/// The assistant files a repository ships on purpose are named in its
/// .gitignore, after the block that keeps the rest out.
#[test]
fn a_shipped_assistant_file_is_un_ignored_by_name() {
    let machine = Machine::new();
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let gitignore = include_str!("../src/templates/community/gitignore.tmpl");
    write(root, ".gitignore", gitignore);
    write(root, "registry/files/AGENTS.md", "a briefing the package hands out\n");
    machine.git(root, &["init", "--quiet"]);
    machine.git(root, &["add", "-f", "registry/files/AGENTS.md"]);
    machine.commit(root, "feat: start", None);

    let (report, _) = machine.doctor(root);
    assert!(report.contains("tracked: registry/files/AGENTS.md"), "{report}");

    write(root, ".gitignore", &format!("{gitignore}!registry/files/AGENTS.md\n"));
    machine.commit(root, "chore: ship the briefing on purpose", None);
    let (report, _) = machine.doctor(root);
    assert!(report.contains("✓ ai-traces"), "{report}");
}

#[test]
fn the_identity_this_machine_commits_as_is_held_to_the_global_one() {
    let machine = Machine::new();
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(root, "README.md", "# demo\n\n> A demo.\n");
    machine.git(root, &["init", "--quiet"]);
    machine.commit(root, "feat: start", None);

    let (report, _) = machine.doctor(root);
    assert!(
        report.contains("✓ identity   commits as `Ada Lovelace <ada@example.com>`, the global identity"),
        "{report}"
    );

    machine.git(root, &["config", "user.name", "lacodda"]);
    let (report, _) = machine.doctor(root);
    assert!(report.contains("! identity"), "{report}");
    assert!(
        report.contains("commits here are signed `lacodda <ada@example.com>`, from .git/config; the global identity is `Ada Lovelace <ada@example.com>`"),
        "{report}"
    );

    // A runner has neither an identity nor an excludes file of its own.
    let out = machine.lyrn().env("CI", "true").arg("doctor").arg(root).output().unwrap();
    let report = String::from_utf8(out.stdout).unwrap();
    assert!(report.contains("- identity   a CI runner has no identity"), "{report}");
}

#[test]
fn a_machine_with_nothing_configured_is_told_so() {
    let machine = Machine::bare();
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    machine.git(root, &["init", "--quiet"]);

    let (report, passed) = machine.doctor(root);
    assert!(!passed, "{report}");
    assert!(report.contains("✗ identity"), "{report}");
    assert!(report.contains("no git identity: commits here would carry a name git guesses"), "{report}");
    assert!(report.contains("no global excludes file"), "{report}");
    assert!(report.contains("- authors    no commits yet"), "{report}");
}

#[test]
fn explain_says_why_and_where_to_read_more() {
    let rules = [
        "standard",
        "readme",
        "mark",
        "actions",
        "msrv",
        "node",
        "audit",
        "ai-traces",
        "authors",
        "identity",
    ];
    for rule in rules {
        let out = common::lyrn().args(["doctor", "--explain", rule]).output().unwrap();
        assert!(out.status.success(), "{rule}");
        let text = String::from_utf8(out.stdout).unwrap();
        assert!(text.starts_with(&format!("{rule} - ")), "{rule}: {text}");
        assert!(
            text.trim_end().ends_with(&format!("More: https://lyrn.lacodda.com/reference/doctor/#{rule}")),
            "{rule}: {text}"
        );
    }
    // A rule that does not exist is refused with the ones that do.
    let out = common::lyrn().args(["doctor", "--explain", "nonsense"]).output().unwrap();
    assert!(!out.status.success());
    let error = String::from_utf8(out.stderr).unwrap();
    assert!(error.contains("ai-traces") && error.contains("identity"), "{error}");
}

/// A path inside a repository examines the repository, from its top.
#[test]
fn a_directory_inside_a_repository_examines_the_whole_of_it() {
    let machine = Machine::new();
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(root, "Cargo.toml", "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2024\"\n");
    write(root, "src/main.rs", "fn main() {}\n");
    machine.git(root, &["init", "--quiet"]);
    machine.commit(root, "feat: start", None);

    let (report, _) = machine.doctor(&root.join("src"));
    assert!(report.contains("Cargo.toml declares no rust-version"), "{report}");
}
