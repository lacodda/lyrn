//! The `docs` form adds to a repository, so what it must not do matters as
//! much as what it writes: it must not change, replace or half-write anything
//! that was there before it.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;

/// A repository as lyrn's other forms leave it: a README, some code, and the
/// `docs/adr/` directory every form writes - the case the form exists for.
fn repository() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::create_dir_all(root.join("docs/adr")).unwrap();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::create_dir_all(root.join(".github/workflows")).unwrap();
    fs::write(root.join("README.md"), "# my-tool\n").unwrap();
    fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();
    fs::write(root.join("docs/adr/0001-record-architecture-decisions.md"), "# ADR 0001\n").unwrap();
    fs::write(root.join(".github/workflows/ci.yml"), "name: CI\n").unwrap();
    dir
}

/// Every file under `root`, with its bytes.
fn snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                out.insert(path.strip_prefix(root).unwrap().to_path_buf(), fs::read(&path).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

fn lyrn(root: &Path) -> Command {
    let mut cmd = Command::cargo_bin("lyrn").unwrap();
    cmd.args(["new", "my-tool", "--form", "docs", "--yes", "--no-hooks", "--repo", "someone/my-tool"])
        .arg("--path")
        .arg(root);
    cmd
}

#[test]
fn it_adds_beside_what_is_there_and_changes_none_of_it() {
    let repo = repository();
    let before = snapshot(repo.path());

    lyrn(repo.path()).assert().success();

    let after = snapshot(repo.path());
    for (path, bytes) in &before {
        assert_eq!(after.get(path), Some(bytes), "{} was changed", path.display());
    }
    let added: Vec<String> = after
        .keys()
        .filter(|p| !before.contains_key(*p))
        .map(|p| p.display().to_string().replace('\\', "/"))
        .collect();
    assert!(added.iter().any(|p| p == "docs/astro.config.mjs"), "no site was written: {added:?}");
    for path in &added {
        assert!(
            path.starts_with("docs/") || path == ".github/workflows/docs.yml",
            "`{path}` is outside docs/ - the form writes the site and its workflow, nothing else"
        );
    }
}

#[test]
fn the_default_destination_is_the_directory_it_is_run_in() {
    let repo = repository();
    Command::cargo_bin("lyrn")
        .unwrap()
        .args(["new", "my-tool", "--form", "docs", "--yes", "--no-hooks", "--repo", "someone/my-tool"])
        .current_dir(repo.path())
        .assert()
        .success();
    assert!(repo.path().join("docs/astro.config.mjs").exists());
    assert!(!repo.path().join("my-tool").exists(), "a directory named after the project was created instead");
}

#[test]
fn a_file_in_the_way_stops_everything_and_every_one_is_named() {
    let repo = repository();
    fs::write(repo.path().join("docs/package.json"), "{\"mine\": true}\n").unwrap();
    fs::write(repo.path().join(".github/workflows/docs.yml"), "name: mine\n").unwrap();
    let before = snapshot(repo.path());

    let output = lyrn(repo.path()).assert().failure().get_output().stderr.clone();
    let stderr = String::from_utf8(output).unwrap();

    assert!(stderr.contains("docs/package.json"), "{stderr}");
    assert!(stderr.contains(".github/workflows/docs.yml"), "{stderr}");
    assert_eq!(snapshot(repo.path()), before, "a refused generation still wrote something");
}

#[test]
fn a_site_of_another_kind_is_refused() {
    let repo = repository();
    fs::write(repo.path().join("mkdocs.yml"), "site_name: mine\n").unwrap();
    let before = snapshot(repo.path());

    lyrn(repo.path()).assert().failure().stderr(predicates::str::contains("mkdocs.yml"));
    assert_eq!(snapshot(repo.path()), before);
}

#[test]
fn a_repository_that_is_not_there_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("nowhere");
    lyrn(&missing).assert().failure().stderr(predicates::str::contains("does not exist"));
    assert!(!missing.exists(), "the missing repository was created");
}

/// A dry run makes the same checks as the real one: promising files that the
/// run would then refuse to write is a promise broken.
#[test]
fn a_dry_run_refuses_what_the_run_would_refuse() {
    let repo = repository();
    fs::write(repo.path().join("docs/package.json"), "{}\n").unwrap();
    lyrn(repo.path()).arg("--dry-run").assert().failure();

    let clean = repository();
    let before = snapshot(clean.path());
    lyrn(clean.path())
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicates::str::contains("Would add"));
    assert_eq!(snapshot(clean.path()), before, "a dry run wrote something");
}

/// A github.io project site lives under the repository's name, which is the
/// `--repo` given - not necessarily the project's.
#[test]
fn the_site_starts_on_github_pages_under_the_repository_name() {
    let repo = repository();
    Command::cargo_bin("lyrn")
        .unwrap()
        .args(["new", "my-tool", "--form", "docs", "--yes", "--no-hooks", "--repo", "someone/tool-docs"])
        .arg("--path")
        .arg(repo.path())
        .assert()
        .success();

    let astro = fs::read_to_string(repo.path().join("docs/astro.config.mjs")).unwrap();
    assert!(astro.contains("new URL('https://someone.github.io/tool-docs/')"), "{astro}");
    assert!(!repo.path().join("docs/public/CNAME").exists(), "a github.io site has no CNAME");

    let not_found = fs::read_to_string(repo.path().join("docs/src/content/docs/404.md")).unwrap();
    assert!(not_found.contains("link: /tool-docs/"), "{not_found}");
}

/// A description is prose, and prose has apostrophes and colons - valid text,
/// and broken JavaScript or YAML when pasted in bare.
#[test]
fn a_description_with_punctuation_survives_both_languages() {
    let repo = repository();
    let description = r#"The tool's manual: "quoted", and a back\slash"#;
    lyrn(repo.path()).args(["--description", description]).assert().success();

    let as_json = r#""The tool's manual: \"quoted\", and a back\\slash""#;
    let astro = fs::read_to_string(repo.path().join("docs/astro.config.mjs")).unwrap();
    assert!(astro.contains(&format!("const description = {as_json};")), "{astro}");
    let index = fs::read_to_string(repo.path().join("docs/src/content/docs/index.md")).unwrap();
    assert!(index.contains(&format!("description: {as_json}")), "{index}");
}

#[test]
fn nothing_is_left_unrendered() {
    let repo = repository();
    lyrn(repo.path()).assert().success();
    for (path, bytes) in snapshot(&repo.path().join("docs")) {
        let Ok(text) = String::from_utf8(bytes) else { continue };
        assert!(!text.contains("{{"), "docs/{} has an unrendered placeholder", path.display());
    }
    let workflow = fs::read_to_string(repo.path().join(".github/workflows/docs.yml")).unwrap();
    for (index, _) in workflow.match_indices("{{") {
        assert_eq!(&workflow[index - 1..index], "$", "the workflow has an unrendered placeholder");
    }
}
