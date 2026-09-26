//! `lyrn init`: a project started in a directory that already exists.

use std::fs;
use std::path::Path;

use assert_cmd::Command;

fn lyrn() -> Command {
    Command::cargo_bin("lyrn").unwrap()
}

fn init(dir: &Path) -> Command {
    let mut cmd = lyrn();
    cmd.args(["init", "--yes", "--no-hooks", "--repo", "owner/demo-tool"]).arg(dir);
    cmd
}

#[test]
fn a_directory_that_exists_gets_the_project_named_after_it() {
    let parent = tempfile::tempdir().unwrap();
    let dir = parent.path().join("demo-tool");
    fs::create_dir(&dir).unwrap();
    // What a clone of a new, empty repository leaves behind.
    fs::create_dir(dir.join(".git")).unwrap();

    init(&dir).args(["--form", "cli"]).assert().success();

    let lyrn_toml = fs::read_to_string(dir.join("lyrn.toml")).unwrap();
    assert!(lyrn_toml.contains("name = \"demo-tool\""), "{lyrn_toml}");
    assert!(lyrn_toml.contains("form = \"cli\""), "{lyrn_toml}");
    assert!(dir.join("src/main.rs").is_file());
}

/// The files a hosting service puts in a new repository are the usual thing
/// in the way. Nothing is written, every one of them is named, and the way
/// round is offered.
#[test]
fn a_file_in_the_way_stops_the_whole_run() {
    let parent = tempfile::tempdir().unwrap();
    let dir = parent.path().join("demo-tool");
    fs::create_dir(&dir).unwrap();
    fs::write(dir.join("README.md"), "# mine").unwrap();
    fs::write(dir.join("LICENSE"), "mine").unwrap();

    let out = init(&dir).args(["--form", "cli"]).assert().failure().get_output().stderr.clone();
    let text = String::from_utf8_lossy(&out);
    assert!(text.contains("README.md") && text.contains("LICENSE"), "{text}");
    assert!(text.contains("lyrn adopt"), "{text}");

    assert_eq!(fs::read_to_string(dir.join("README.md")).unwrap(), "# mine");
    assert!(!dir.join("Cargo.toml").exists(), "a refused run wrote files anyway");
}

#[test]
fn a_directory_name_that_is_not_a_project_name_asks_for_one() {
    let parent = tempfile::tempdir().unwrap();
    let dir = parent.path().join("Demo Tool");
    fs::create_dir(&dir).unwrap();

    let out = init(&dir).args(["--form", "cli"]).assert().failure().get_output().stderr.clone();
    assert!(String::from_utf8_lossy(&out).contains("--name"));

    init(&dir).args(["--form", "cli", "--name", "demo-tool"]).assert().success();
    assert!(fs::read_to_string(dir.join("lyrn.toml")).unwrap().contains("name = \"demo-tool\""));
}

#[test]
fn a_directory_that_does_not_exist_is_for_lyrn_new() {
    let parent = tempfile::tempdir().unwrap();
    let out = init(&parent.path().join("nowhere")).assert().failure().get_output().stderr.clone();
    assert!(String::from_utf8_lossy(&out).contains("lyrn new"));
}

#[test]
fn a_dry_run_shows_the_tree_and_writes_nothing() {
    let parent = tempfile::tempdir().unwrap();
    let dir = parent.path().join("demo-tool");
    fs::create_dir(&dir).unwrap();

    let out = init(&dir).args(["--form", "cli", "--dry-run"]).assert().success().get_output().stdout.clone();
    let text = String::from_utf8_lossy(&out);
    assert!(text.contains("Would create") && text.contains("main.rs"), "{text}");
    assert_eq!(fs::read_dir(&dir).unwrap().count(), 0, "--dry-run wrote into the directory");
}
