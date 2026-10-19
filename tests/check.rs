//! `lyrn check`, run the way a gate runs it: on repositories made for the
//! purpose, each repeating one lesson of the line, and on lyrn's own.
//!
//! The program whose `--help` the `commands` check reads is lyrn itself,
//! copied into the fixture's `target/debug` under the fixture's name: a real
//! clap program with real commands, and no build to wait for.

use std::fs;
use std::path::Path;
use std::time::SystemTime;

mod common;

const NAME: &str = "demo";
const VERSION: &str = "1.2.0";
const DESCRIPTION: &str = "Count the words in a file, fast";

fn write(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// The commands lyrn's help lists: what the stand-in program has.
fn commands() -> Vec<String> {
    let help = String::from_utf8(common::lyrn().arg("--help").output().unwrap().stdout).unwrap();
    help.lines()
        .skip_while(|l| !l.starts_with("Commands:"))
        .skip(1)
        .take_while(|l| !l.trim().is_empty())
        .filter_map(|l| l.split_whitespace().next().map(str::to_string))
        .filter(|c| c != "help")
        .collect()
}

/// A repository whose shopfronts agree: a crate with an npm wrapper, a
/// changelog, a README, a Starlight site on a project site, installers, and
/// a built program.
fn repository() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(
        root,
        "Cargo.toml",
        &format!(
            "[package]\nname = \"{NAME}\"\nversion = \"{VERSION}\"\nedition = \"2024\"\ndescription = \"{DESCRIPTION}\"\nrepository = \"https://github.com/someone/{NAME}\"\nhomepage = \"https://someone.github.io/{NAME}/\"\n\n[dependencies]\nclap = \"4\"\n"
        ),
    );
    write(root, "src/main.rs", "fn main() {}\n");
    write(
        root,
        "Cargo.lock",
        &format!("version = 4\n\n[[package]]\nname = \"{NAME}\"\nversion = \"{VERSION}\"\n"),
    );
    write(
        root,
        "npm/package.json",
        &format!(
            "{{\n  \"name\": \"{NAME}-cli\",\n  \"version\": \"{VERSION}\",\n  \"{NAME}\": {{ \"binary\": \"v{VERSION}\" }},\n  \"description\": \"{DESCRIPTION}\",\n  \"homepage\": \"https://someone.github.io/{NAME}\"\n}}\n"
        ),
    );
    write(root, "CHANGELOG.md", &format!("# Changelog\n\n## [{VERSION}] - 2026-10-19\n\n- Counting\n"));
    write(
        root,
        "README.md",
        &format!(
            "<p align=\"center\"><img src=\"https://raw.githubusercontent.com/someone/{NAME}/main/assets/banner.svg\" alt=\"{NAME}\"></p>\n\n> {DESCRIPTION}.\n\n```console\n$ {NAME} new notes\n$ {NAME} template check ./mine\n```\n\n[Docs](https://someone.github.io/{NAME}/)\n"
        ),
    );
    write(
        root,
        "docs/astro.config.mjs",
        &format!(
            "const description = '{DESCRIPTION}.';\nexport default defineConfig({{\n\tsite: 'https://someone.github.io',\n\tbase: '/{NAME}',\n\tintegrations: [starlight({{ title: '{NAME}', description }})],\n}});\n"
        ),
    );
    let mut reference = String::from("---\ntitle: Commands\n---\n\n");
    for command in commands() {
        reference.push_str(&format!("`{NAME} {command}` does a thing.\n\n"));
    }
    write(root, "docs/src/content/docs/reference/cli.md", &reference);
    write(
        root,
        "tools/install.sh",
        &format!("REPO=\"someone/{NAME}\"\n# no prebuilt binary? `cargo install {NAME}`\ncurl -fsSL \"https://github.com/$REPO/releases/latest\"\n"),
    );
    build(root);
    dir
}

/// The stand-in program, built after every source so it reads as fresh.
fn build(root: &Path) {
    let program = root.join("target/debug").join(format!("{NAME}{}", std::env::consts::EXE_SUFFIX));
    fs::create_dir_all(program.parent().unwrap()).unwrap();
    fs::copy(env!("CARGO_BIN_EXE_lyrn"), &program).unwrap();
    // A copy keeps the original's time on some systems: the program has to
    // be newer than the sources, or the check builds it again.
    fs::File::options().write(true).open(&program).unwrap().set_modified(SystemTime::now()).unwrap();
}

/// `lyrn check <root>`: the report, and whether it passed.
fn check(root: &Path) -> (String, bool) {
    // The stand-in program was just written, and on Linux running it while
    // another test's fork still holds it open fails with "Text file busy".
    // That flake, and only it, is tried again.
    for _ in 0..5 {
        let out = common::lyrn().arg("check").arg(root).output().unwrap();
        let report = String::from_utf8(out.stdout).unwrap();
        if report.contains("Text file busy") {
            std::thread::sleep(std::time::Duration::from_millis(200));
            continue;
        }
        assert!(out.stderr.is_empty(), "{}", String::from_utf8_lossy(&out.stderr));
        return (report, out.status.success());
    }
    panic!("the stand-in program stayed busy");
}

#[test]
fn shopfronts_that_agree_pass() {
    let repo = repository();
    let (report, passed) = check(repo.path());
    assert!(passed, "{report}");
    for name in ["versions", "changelog", "descriptions", "readme", "commands", "site", "installers"] {
        assert!(report.lines().any(|l| l.starts_with(&format!("✓ {name}"))), "{name} did not pass:\n{report}");
    }
    assert!(report.contains("4 places say 1.2.0"), "{report}");
}

/// Each lesson the line learnt, repeated once: the check that holds it fails,
/// with the finding that names it, and the others stay green.
#[test]
fn each_lesson_unlearnt_fails_its_check() {
    type Break = fn(&Path);
    let cases: &[(&str, &str, Break)] = &[
        ("versions", "npm/package.json says 1.1.0", |root| {
            let text = fs::read_to_string(root.join("npm/package.json")).unwrap();
            fs::write(
                root.join("npm/package.json"),
                text.replacen("\"version\": \"1.2.0\"", "\"version\": \"1.1.0\"", 1),
            )
            .unwrap();
        }),
        ("versions", "pins the binary at v1.1.0", |root| {
            let text = fs::read_to_string(root.join("npm/package.json")).unwrap();
            fs::write(root.join("npm/package.json"), text.replace("\"binary\": \"v1.2.0\"", "\"binary\": \"v1.1.0\"")).unwrap();
        }),
        ("versions", "Cargo.lock still records demo 1.1.0", |root| {
            write(root, "Cargo.lock", "version = 4\n\n[[package]]\nname = \"demo\"\nversion = \"1.1.0\"\n");
        }),
        ("versions", "release.toml does not list npm/package.json", |root| {
            write(root, "release.toml", "[project]\nname = \"demo\"\nmanifests = [\"Cargo.toml\"]\n");
        }),
        ("changelog", "CHANGELOG.md has no section for 1.2.0", |root| {
            write(root, "CHANGELOG.md", "# Changelog\n\n## [1.1.0] - 2026-10-01\n");
        }),
        ("descriptions", "describes the site as", |root| {
            let text = fs::read_to_string(root.join("docs/astro.config.mjs")).unwrap();
            fs::write(
                root.join("docs/astro.config.mjs"),
                text.replace("Count the words in a file, fast.", "The documentation of Demo."),
            )
            .unwrap();
        }),
        ("descriptions", "npm/package.json says", |root| {
            let text = fs::read_to_string(root.join("npm/package.json")).unwrap();
            fs::write(root.join("npm/package.json"), text.replace("in a file, fast", "in a file")).unwrap();
        }),
        ("readme", "npm/README.md is a second README", |root| write(root, "npm/README.md", "# demo\n")),
        ("readme", "`assets/logo.svg` is relative", |root| {
            let text = fs::read_to_string(root.join("README.md")).unwrap();
            fs::write(root.join("README.md"), format!("{text}\n![logo](assets/logo.svg)\n")).unwrap();
        }),
        ("commands", "shows `demo frobnicate`", |root| {
            let text = fs::read_to_string(root.join("README.md")).unwrap();
            fs::write(root.join("README.md"), text.replace("$ demo new notes", "$ demo frobnicate notes")).unwrap();
        }),
        ("commands", "shows `demo template frobnicate`", |root| {
            let text = fs::read_to_string(root.join("README.md")).unwrap();
            fs::write(root.join("README.md"), text.replace("template check", "template frobnicate")).unwrap();
        }),
        ("commands", "`demo doctor` has no page", |root| {
            let text = fs::read_to_string(root.join("docs/src/content/docs/reference/cli.md")).unwrap();
            fs::write(root.join("docs/src/content/docs/reference/cli.md"), text.replace("`demo doctor`", "`doctor`")).unwrap();
        }),
        ("site", "keeps `base: '/demo'`", |root| {
            write(root, "docs/public/CNAME", "demo.example.com\n");
            let text = fs::read_to_string(root.join("docs/astro.config.mjs")).unwrap();
            fs::write(
                root.join("docs/astro.config.mjs"),
                text.replace("https://someone.github.io", "https://demo.example.com"),
            )
            .unwrap();
        }),
        ("site", "sends readers to https://github.com/someone/demo", |root| {
            let text = fs::read_to_string(root.join("Cargo.toml")).unwrap();
            fs::write(
                root.join("Cargo.toml"),
                text.replace(
                    "homepage = \"https://someone.github.io/demo/\"",
                    "homepage = \"https://github.com/someone/demo\"",
                ),
            )
            .unwrap();
        }),
        ("installers", "asks api.github.com", |root| {
            write(
                root,
                "tools/install.ps1",
                "$Repo = 'someone/demo'\nInvoke-RestMethod https://api.github.com/repos/$Repo/releases/latest\n",
            );
        }),
        ("installers", "suggests `cargo install demo-cli`", |root| {
            let text = fs::read_to_string(root.join("tools/install.sh")).unwrap();
            fs::write(root.join("tools/install.sh"), text.replace("cargo install demo", "cargo install demo-cli")).unwrap();
        }),
        ("installers", "does not name someone/demo", |root| {
            write(root, "tools/install.sh", "REPO=\"someone-else/other\"\n");
        }),
    ];
    for (shop, finding, lesson) in cases {
        let repo = repository();
        lesson(repo.path());
        build(repo.path());
        let (report, passed) = check(repo.path());
        assert!(!passed, "{shop}: `{finding}` passed:\n{report}");
        assert!(report.lines().any(|l| l == format!("✗ {shop}")), "{shop} did not fail:\n{report}");
        assert!(report.contains(finding), "{shop}: no `{finding}` in:\n{report}");
        let failed: Vec<&str> = report.lines().filter(|l| l.starts_with("✗ ")).collect();
        assert_eq!(failed, [format!("✗ {shop}")], "only {shop} should fail:\n{report}");
    }
}

/// A check reads; it never writes - not even the lockfile a build would
/// bring up to date.
#[test]
fn a_check_writes_nothing() {
    let repo = repository();
    write(repo.path(), "Cargo.lock", "version = 4\n\n[[package]]\nname = \"demo\"\nversion = \"1.1.0\"\n");
    let before = fs::read(repo.path().join("Cargo.lock")).unwrap();
    let (report, passed) = check(repo.path());
    assert!(!passed, "{report}");
    assert_eq!(fs::read(repo.path().join("Cargo.lock")).unwrap(), before, "the check rewrote Cargo.lock");
}

#[test]
fn explain_says_why_and_where_to_read_more() {
    let out = common::lyrn().args(["check", "--explain", "site"]).output().unwrap();
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.starts_with("site - The documentation site agrees"), "{text}");
    assert!(text.contains("unstyled HTML"), "{text}");
    assert!(text.trim_end().ends_with("More: https://lyrn.lacodda.com/reference/check/#site"), "{text}");
}

/// The shopfronts of lyrn itself: what `tests/release_consistency.rs` held
/// test by test before the command existed.
#[test]
fn lyrn_s_own_shopfronts_agree() {
    let (report, passed) = check(Path::new(env!("CARGO_MANIFEST_DIR")));
    assert!(passed, "{report}");
}

/// `lyrn check --explain <check>` sends the reader to the check's section of
/// the reference page, by the check's name.
#[test]
fn every_check_has_a_section_on_its_page() {
    let refusal = common::lyrn().args(["check", "--explain", "no-such-check"]).output().unwrap();
    let refusal = String::from_utf8(refusal.stderr).unwrap();
    let listed = refusal
        .split("[possible values: ")
        .nth(1)
        .and_then(|rest| rest.split(']').next())
        .expect("clap lists the checks it accepts");
    let checks: Vec<&str> = listed.split(", ").collect();
    assert!(checks.len() >= 7, "{checks:?}");
    let page = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/src/content/docs/reference/check.md")).unwrap();
    for check in checks {
        assert!(
            page.contains(&format!("\n### {check}\n")),
            "docs/reference/check.md has no `### {check}` section"
        );
    }
}
