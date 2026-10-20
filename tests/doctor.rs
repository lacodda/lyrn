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
        "assets",
        "language",
        "private",
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

/// A directory made canonical before anything is put in it, so the path a
/// file names and the path git reports are the same spelling: no `\\?\`, no
/// 8.3 short names on Windows, no `/private` behind `/var` on macOS.
fn canonical_dir(dir: &tempfile::TempDir) -> PathBuf {
    let path = fs::canonicalize(dir.path()).unwrap();
    let text = path.display().to_string();
    PathBuf::from(text.strip_prefix(r"\\?\").unwrap_or(&text))
}

/// A square PNG of one colour.
fn png_of(size: u32, rgba: [u8; 4]) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, size, size);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        let data: Vec<u8> = (0..size * size).flat_map(|_| rgba).collect();
        writer.write_image_data(&data).unwrap();
    }
    out
}

/// An `.ico` of these images, in this order.
fn ico_of(images: &[(u32, Vec<u8>)]) -> Vec<u8> {
    let mut out = vec![0, 0, 1, 0];
    out.extend_from_slice(&(images.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * images.len();
    for (size, image) in images {
        let side = if *size >= 256 { 0 } else { *size as u8 };
        out.extend_from_slice(&[side, side, 0, 0, 1, 0, 32, 0]);
        out.extend_from_slice(&(image.len() as u32).to_le_bytes());
        out.extend_from_slice(&(offset as u32).to_le_bytes());
        offset += image.len();
    }
    for (_, image) in images {
        out.extend_from_slice(image);
    }
    out
}

/// The sizes an `.ico` lists, in its own order.
fn ico_sizes(ico: &[u8]) -> Vec<u32> {
    let count = u16::from_le_bytes([ico[4], ico[5]]) as usize;
    (0..count)
        .map(|i| if ico[6 + 16 * i] == 0 { 256 } else { u32::from(ico[6 + 16 * i]) })
        .collect()
}

/// The line's plate, under M and L, and a product's colour, filling S.
const PLATE: [u8; 4] = [0x1B, 0x21, 0x26, 255];
const TILE: [u8; 4] = [0x6D, 0x7B, 0xF2, 255];

/// A repository of the line's lessons about the mark: the three masters, a
/// site's favicon a version behind, an `.ico` listed smallest first.
fn repository_with_a_stale_mark(machine: &Machine, root: &Path) {
    write(root, "README.md", "# demo\n\n> A demo.\n");
    write(root, ".gitignore", "target/\n");
    write(root, "assets/logo.svg", "<svg><!-- L --></svg>\n");
    write(root, "assets/logo-m.svg", "<svg><!-- M --></svg>\n");
    write(root, "assets/logo-s.svg", "<svg><!-- S --></svg>\n");
    write(root, "docs/public/favicon.svg", "<svg><!-- last month's S --></svg>\n");
    write(root, "docs/package.json", "{\n  \"name\": \"demo-docs\",\n  \"private\": true\n}\n");
    write(root, "docs/pnpm-lock.yaml", "");
    // Two releases glued together, as a body template that ends without a
    // blank line writes them.
    write(
        root,
        "CHANGELOG.md",
        "# Changelog\n\n## [0.2.0] - 2026-10-02\n\n- Two\n## [0.1.0] - 2026-10-01\n\n- One\n",
    );
    let ico = ico_of(&[
        (16, png_of(16, TILE)),
        (24, png_of(24, TILE)),
        (32, png_of(32, PLATE)),
        (48, png_of(48, PLATE)),
        (256, png_of(256, PLATE)),
    ]);
    fs::write(root.join("assets/icon.ico"), ico).unwrap();
    machine.git(root, &["init", "--quiet"]);
    machine.commit(root, "feat: start", None);
}

#[test]
fn many_repositories_are_a_table_and_json_is_one_document() {
    let machine = Machine::new();
    let dir = tempfile::tempdir().unwrap();
    let good = dir.path().join("good");
    machine
        .lyrn()
        .args([
            "new",
            "good",
            "--form",
            "cli",
            "--yes",
            "--no-hooks",
            "--accent",
            "#6D7BF2",
            "--repo",
            "owner/good",
            "--path",
        ])
        .arg(&good)
        .assert()
        .success();
    write(&good, "Cargo.lock", "");
    machine.git(&good, &["init", "--quiet"]);
    machine.commit(&good, "feat: start", None);
    let bad = dir.path().join("bad");
    fs::create_dir_all(&bad).unwrap();
    repository_with_a_stale_mark(&machine, &bad);

    let out = machine.lyrn().arg("doctor").arg(&good).arg(&bad).output().unwrap();
    let report = String::from_utf8(out.stdout).unwrap();
    assert!(!out.status.success(), "one of them fails:\n{report}");
    assert!(report.starts_with("lyrn doctor: 2 repositories\n"), "{report}");
    let header = report.lines().find(|l| l.trim_start().starts_with("standard readme")).unwrap_or_default();
    let names: Vec<&str> = header.split_whitespace().collect();
    assert_eq!(
        names,
        [
            "standard",
            "readme",
            "changelog",
            "mark",
            "assets",
            "language",
            "private",
            "actions",
            "msrv",
            "node",
            "audit",
            "ai-traces",
            "authors",
            "identity"
        ]
    );
    let row = |name: &str| report.lines().find(|l| l.starts_with(&format!("{name} "))).unwrap_or_default().to_string();
    assert!(!row("good").contains('✗') && !row("good").contains('!'), "{report}");
    // `assets` is the fifth column, and the stale favicon fails it.
    let bad_row = row("bad");
    let marks: Vec<&str> = bad_row.split_whitespace().skip(1).collect();
    assert_eq!(marks[4], "✗", "{report}");
    assert!(report.contains("1 fails, 1 passes; `lyrn doctor --fix` puts 7 findings right"), "{report}");
    assert!(
        report
            .lines()
            .any(|l| l.starts_with("stack ") && l.contains("edition") && l.contains("updated")),
        "{report}"
    );
    assert!(report.lines().any(|l| l.starts_with("standard ") && l.contains("weekly")), "{report}");

    // The same examination as one document.
    let out = machine.lyrn().args(["doctor", "--json"]).arg(&good).arg(&bad).output().unwrap();
    let document: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let repositories = document["repositories"].as_array().unwrap();
    assert_eq!(repositories.len(), 2);
    assert_eq!(repositories[0]["name"], "good");
    assert_eq!(repositories[0]["result"], "pass");
    assert_eq!(repositories[1]["result"], "fail");
    let rules = repositories[1]["rules"].as_array().unwrap();
    assert_eq!(rules.len(), 14);
    let assets = rules.iter().find(|r| r["rule"] == "assets").unwrap();
    assert_eq!(assets["result"], "fail");
    let favicon = assets["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["what"].as_str().unwrap().contains("favicon"))
        .unwrap();
    assert_eq!(favicon["repair"]["path"], "docs/public/favicon.svg");
    assert!(document["standard"]["pnpm"].as_str().unwrap().starts_with("10."), "{document}");

    // One repository is the same shape: whatever reads it reads one form.
    let out = machine.lyrn().args(["doctor", "--json"]).arg(&good).output().unwrap();
    let document: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(document["repositories"].as_array().unwrap().len(), 1);
}

#[test]
fn fix_writes_the_repairs_and_examines_again() {
    let machine = Machine::new();
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    repository_with_a_stale_mark(&machine, root);

    let (report, _) = machine.doctor(root);
    for expected in [
        ".gitignore does not keep out",
        "docs/package.json names no packageManager",
        "docs/pnpm-lock.yaml: no workflow audits it",
        "docs/public/favicon.svg is not assets/logo-s.svg",
        "assets/icon.ico lists its images 16, 24, 32, 48, 256",
        "docs/pnpm-workspace.yaml lets a script run against node_modules older than docs/pnpm-lock.yaml",
        "CHANGELOG.md: 1 release starts straight after the line above - the first, `## [0.1.0] - 2026-10-01`, at line 6",
        "`lyrn doctor --fix` puts 7 findings right",
    ] {
        assert!(report.contains(expected), "the report does not say `{expected}`:\n{report}");
    }

    let out = machine.lyrn().args(["doctor", "--fix"]).arg(root).output().unwrap();
    let report = String::from_utf8(out.stdout).unwrap();
    for written in [
        "Fixed:\n",
        "  ✓ .gitignore: ends with the standard's block of assistant files\n",
        "  ✓ docs/package.json: pins pnpm@",
        "  ✓ .github/workflows/docs-audit.yml: audits the documentation site's packages",
        "  ✓ docs/public/favicon.svg: copied from assets/logo-s.svg\n",
        "  ✓ assets/icon.ico: put its largest image first\n",
        "  ✓ docs/pnpm-workspace.yaml: has pnpm install what fell behind the lockfile before a script runs\n",
        "  ✓ CHANGELOG.md: puts a blank line before every release\n",
    ] {
        assert!(report.contains(written), "the report does not say `{written}`:\n{report}");
    }
    // Examined again after writing: what was put right is no longer found.
    for gone in [
        "does not keep out",
        "names no packageManager",
        "no workflow audits it",
        "is not assets/logo-s.svg",
        "lists its images",
        "lets a script run against",
        "straight after the line above",
    ] {
        assert!(!report.contains(gone), "`{gone}` is still found after --fix:\n{report}");
    }

    // And the files say so themselves.
    let gitignore = fs::read_to_string(root.join(".gitignore")).unwrap();
    assert!(gitignore.starts_with("target/\n") && gitignore.ends_with("openspec/\n"), "{gitignore}");
    let package: serde_json::Value = serde_json::from_str(&fs::read_to_string(root.join("docs/package.json")).unwrap()).unwrap();
    assert!(package["packageManager"].as_str().unwrap().starts_with("pnpm@"));
    assert!(root.join(".github/workflows/docs-audit.yml").is_file());
    assert_eq!(
        fs::read(root.join("docs/public/favicon.svg")).unwrap(),
        fs::read(root.join("assets/logo-s.svg")).unwrap()
    );
    assert_eq!(ico_sizes(&fs::read(root.join("assets/icon.ico")).unwrap()), [256, 48, 32, 24, 16]);
    assert!(
        fs::read_to_string(root.join("docs/pnpm-workspace.yaml"))
            .unwrap()
            .contains("\nverifyDepsBeforeRun: install\n")
    );
    assert_eq!(
        fs::read_to_string(root.join("CHANGELOG.md")).unwrap(),
        "# Changelog\n\n## [0.2.0] - 2026-10-02\n\n- Two\n\n## [0.1.0] - 2026-10-01\n\n- One\n"
    );

    // Nothing is left for a second run to write.
    let out = machine.lyrn().args(["doctor", "--fix"]).arg(root).output().unwrap();
    assert!(!String::from_utf8(out.stdout).unwrap().contains("Fixed:"));
}

#[test]
fn an_icon_cut_from_one_level_fails_at_the_sizes_it_does_not_read() {
    let machine = Machine::new();
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(root, "assets/logo.svg", "<svg><!-- L --></svg>\n");
    write(root, "assets/logo-m.svg", "<svg><!-- M --></svg>\n");
    write(root, "assets/logo-s.svg", "<svg><!-- S --></svg>\n");
    // The whole container from S, the way kilna and nooma once shipped it.
    let ico = ico_of(&[
        (256, png_of(256, TILE)),
        (48, png_of(48, TILE)),
        (32, png_of(32, TILE)),
        (24, png_of(24, TILE)),
        (16, png_of(16, TILE)),
    ]);
    fs::write(root.join("assets/icon.ico"), ico).unwrap();
    fs::write(root.join("assets/apple-touch-icon.png"), png_of(180, TILE)).unwrap();
    machine.git(root, &["init", "--quiet"]);
    machine.commit(root, "feat: start", None);

    let (report, _) = machine.doctor(root);
    assert!(
        report.contains("assets/icon.ico: the 256px, 48px, 32px images are the wrong level of the mark"),
        "{report}"
    );
    assert!(report.contains("assets/apple-touch-icon.png is the filled S tile at 180px"), "{report}");
}

#[test]
fn what_ships_speaks_english_and_translations_live_in_their_places() {
    let machine = Machine::new();
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(root, "src/lib.rs", "// Этот модуль считает слова в тексте\npub fn count() {}\n");
    write(root, "README.md", "# demo\n\n> A demo that counts words.\n\nRun the сount command.\n");
    write(root, "docs/ru/README.md", "Это перевод документации на русский язык.\n");
    write(
        root,
        "tests/data.rs",
        "#[test]\nfn t() {\n    assert_eq!(f(\"Это строка на русском языке\"), 1);\n}\n",
    );
    machine.git(root, &["init", "--quiet"]);
    machine.commit(root, "feat: start", None);
    machine.commit(root, "исправил сборку проекта", None);

    let (report, _) = machine.doctor(root);
    for expected in [
        "✗ language",
        "Cyrillic prose in 1 file: src/lib.rs:1",
        "1 word typed partly in a Cyrillic layout: `сount` (README.md:5)",
        "1 commit's subject is not written in English",
    ] {
        assert!(report.contains(expected), "the report does not say `{expected}`:\n{report}");
    }
    assert!(!report.contains("docs/ru/README.md") && !report.contains("tests/data.rs"), "{report}");
}

#[test]
fn nothing_from_this_computer_is_let_into_the_repository() {
    let machine = Machine::new();
    let dir = tempfile::tempdir().unwrap();
    let base = canonical_dir(&dir);
    let projects = base.join("projects");
    let root = projects.join("demo");
    fs::create_dir_all(projects.join("other")).unwrap();
    let home = base.join("Users").join("adalind");
    fs::create_dir_all(&home).unwrap();

    let other = projects.join("other").join("assets").display().to_string().replace('\\', "/");
    let nowhere = projects.join("nowhere").join("assets").display().to_string().replace('\\', "/");
    write(
        &root,
        "scripts/export.mjs",
        &format!("const ASSETS = \"{other}\"\nconst EXAMPLE = \"{nowhere}\"\n"),
    );
    write(
        &root,
        "docs/notes.md",
        "Copied from C:\\Users\\adalind\\Desktop and /home/adalind/src.\n\nBuilt on TESTBOX-7731.\n",
    );
    machine.git(&root, &["init", "--quiet"]);
    machine.commit(&root, "feat: start", None);

    let run = |ci: bool| {
        let mut cmd = machine.lyrn();
        cmd.env("USERPROFILE", &home)
            .env("HOME", &home)
            .env("COMPUTERNAME", "TESTBOX-7731")
            .env("HOSTNAME", "TESTBOX-7731");
        if ci {
            cmd.env("CI", "true");
        }
        String::from_utf8(cmd.arg("doctor").arg(&root).output().unwrap().stdout).unwrap()
    };
    let report = run(false);
    for expected in [
        "✗ private",
        "the home of `adalind`, this computer's account, in docs/notes.md:1",
        "naming other - in scripts/export.mjs:1",
        "`TESTBOX-7731`, this computer's name, in docs/notes.md:3",
    ] {
        assert!(report.contains(expected), "the report does not say `{expected}`:\n{report}");
    }
    assert!(
        !report.contains("nowhere") && !report.contains("export.mjs:2"),
        "a made-up place is not this computer:\n{report}"
    );

    // A runner's home and name are nobody's.
    assert!(run(true).contains("- private    a CI runner's home and name are not yours"));
}
