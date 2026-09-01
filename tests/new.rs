//! The gate on the template: what `lyrn new` actually puts on disk.
//!
//! These run the built binary rather than the library, so they check what a
//! user gets, not what the internals believe they produce.

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use predicates::prelude::*;

fn lyrn() -> Command {
    Command::cargo_bin("lyrn").unwrap()
}

/// Generate a project into a fresh temporary directory and hand back its root.
fn generate(name: &str, extra: &[&str]) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join(name);
    let mut cmd = lyrn();
    cmd.arg("new").arg(name).arg("--yes").arg("--no-hooks").arg("--path").arg(&root);
    for arg in extra {
        cmd.arg(arg);
    }
    cmd.assert().success();
    (dir, root)
}

#[test]
fn writes_the_files_the_standard_asks_for() {
    let (_dir, root) = generate("demo-app", &[]);

    // The standard is carried by files, not described in prose.
    for expected in [
        "package.json",
        "vite.config.ts",
        "vitest.config.ts",
        "tsconfig.json",
        "eslint.config.js",
        "index.html",
        "src/main.tsx",
        "src/App.tsx",
        "src/App.test.tsx",
        "src/styles.css",
        "src/lib/utils.ts",
        "components.json",
        "README.md",
        "LICENSE",
        "CHANGELOG.md",
        "cliff.toml",
        ".gitignore",
        ".editorconfig",
        ".gitattributes",
        ".github/workflows/ci.yml",
        "docs/adr/0001-record-architecture-decisions.md",
        "docs/adr/README.md",
        "lyrn.toml",
    ] {
        assert!(root.join(expected).is_file(), "missing from the generated project: {expected}");
    }
}

#[test]
fn no_generated_file_still_holds_a_placeholder() {
    let (_dir, root) = generate("demo-app", &[]);

    // A template that renders half a value produces a project that looks fine
    // and is wrong. The two files that legitimately carry `{{ }}` are the ones
    // the manifest marks verbatim, because Tera and GitHub Actions own that
    // syntax for their own purposes.
    let verbatim = [Path::new("cliff.toml"), Path::new(".github/workflows/ci.yml")];

    for entry in walk(&root) {
        let relative = entry.strip_prefix(&root).unwrap();
        if verbatim.contains(&relative) {
            continue;
        }
        let contents = fs::read_to_string(&entry).unwrap();
        assert!(!contents.contains("{{"), "{} still contains an unrendered placeholder", relative.display());
    }
}

#[test]
fn the_name_reaches_the_places_that_carry_it() {
    let (_dir, root) = generate("demo-app", &[]);

    let package = fs::read_to_string(root.join("package.json")).unwrap();
    assert!(package.contains("\"name\": \"demo-app\""));

    let readme = fs::read_to_string(root.join("README.md")).unwrap();
    assert!(readme.starts_with("# Demo App"));

    let html = fs::read_to_string(root.join("index.html")).unwrap();
    assert!(html.contains("<title>Demo App</title>"));
}

#[test]
fn the_accent_of_a_product_of_the_line_lands_in_the_theme() {
    let (_dir, root) = generate("demo-app", &["--accent", "kilna"]);

    let styles = fs::read_to_string(root.join("src/styles.css")).unwrap();
    assert!(styles.contains("--accent-base: #D9569E;"), "the accent is not in the theme");
    // The theme itself comes from dowel; the project only states its colour.
    assert!(styles.contains("@import 'dowel-ui/theme.css';"));
}

#[test]
fn a_literal_colour_is_accepted_too() {
    let (_dir, root) = generate("demo-app", &["--accent", "#a1b2c3"]);
    let styles = fs::read_to_string(root.join("src/styles.css")).unwrap();
    assert!(styles.contains("--accent-base: #A1B2C3;"));
}

#[test]
fn records_what_it_was_generated_from() {
    let (_dir, root) = generate("demo-app", &[]);

    // `doctor` and `upgrade` read this file to know what standard the
    // repository is meant to be holding to.
    let stamp = fs::read_to_string(root.join("lyrn.toml")).unwrap();
    assert!(stamp.contains("name = \"demo-app\""));
    assert!(stamp.contains("form = \"spa\""));
    assert!(stamp.contains(&format!("version = \"{}\"", env!("CARGO_PKG_VERSION"))));
    assert!(stamp.contains("standard = \"2026.09\""));
}

#[test]
fn the_verbatim_files_keep_their_own_syntax() {
    let (_dir, root) = generate("demo-app", &[]);

    // git-cliff's Tera template and Actions' expressions both use `{{ }}`, and
    // substituting into them would break the file rather than fill it in.
    let cliff = fs::read_to_string(root.join("cliff.toml")).unwrap();
    assert!(cliff.contains("{{"), "cliff.toml lost its Tera template");

    let ci = fs::read_to_string(root.join(".github/workflows/ci.yml")).unwrap();
    assert!(ci.contains("${{ github.workflow }}"), "the workflow lost its expressions");
}

#[test]
fn dry_run_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("demo-app");

    lyrn()
        .args(["new", "demo-app", "--yes", "--no-hooks", "--dry-run", "--path"])
        .arg(&root)
        .assert()
        .success()
        .stdout(predicate::str::contains("package.json"));

    assert!(!root.exists(), "--dry-run created the project anyway");
}

#[test]
fn refuses_a_destination_that_is_not_empty() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("demo-app");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("occupied.txt"), "mine").unwrap();

    lyrn()
        .args(["new", "demo-app", "--yes", "--no-hooks", "--path"])
        .arg(&root)
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists and is not empty"));

    // The file that was there is still there.
    assert_eq!(fs::read_to_string(root.join("occupied.txt")).unwrap(), "mine");
}

#[test]
fn refuses_a_name_a_package_could_not_have() {
    lyrn()
        .args(["new", "Demo App", "--yes", "--no-hooks"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("lowercase"));
}

#[test]
fn lists_its_forms() {
    lyrn().arg("forms").assert().success().stdout(predicate::str::contains("spa"));
}

/// Every file under `root`, recursively.
fn walk(root: &Path) -> Vec<std::path::PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else {
                found.push(path);
            }
        }
    }
    found
}
