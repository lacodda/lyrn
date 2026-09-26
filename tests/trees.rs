//! The tree of every form and every combination of its add-ons, held to a
//! snapshot in `tests/trees/`.
//!
//! The other gates ask whether what is generated works. This one asks whether
//! it is what was meant: a file that moved, vanished or arrived with an
//! unrelated add-on still builds, and nothing else in the gate would notice.
//! A change to what a form writes has to show up here as a changed snapshot,
//! in the same commit, where a reviewer reads it as a tree.
//!
//! After a deliberate change: `LYRN_BLESS=1 cargo test --test trees`, then
//! read the diff of `tests/trees/` before committing it.

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;

mod common;
use common::{FORMS, combinations};

fn snapshots() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("trees")
}

fn snapshot_name(form: &str, addons: &[&str]) -> String {
    let mut name = form.to_string();
    for addon in addons {
        name.push('+');
        name.push_str(addon);
    }
    format!("{name}.txt")
}

/// What `lyrn new --dry-run` prints for this form and these add-ons.
fn dry_run(form: &str, addons: &[&str]) -> String {
    let dir = tempfile::tempdir().unwrap();
    let mut cmd = Command::cargo_bin("lyrn").unwrap();
    cmd.arg("new")
        .arg("demo-tool")
        .args(["--form", form, "--yes", "--dry-run", "--repo", "owner/demo-tool"]);
    if form == "plugin" {
        cmd.args(["--host", "kilna"]);
    }
    if !addons.is_empty() {
        cmd.arg("--with").arg(addons.join(","));
    }
    // A relative destination, so the heading names it the same way on every
    // machine. The docs form adds to the directory it is run in, which has to
    // exist.
    if form == "docs" {
        cmd.current_dir(dir.path());
    } else {
        cmd.current_dir(dir.path()).args(["--path", "demo-tool"]);
    }
    let output = cmd.assert().success().get_output().stdout.clone();
    String::from_utf8(output).unwrap().replace("\r\n", "\n")
}

#[test]
fn every_form_writes_the_tree_its_snapshot_shows() {
    let bless = std::env::var_os("LYRN_BLESS").is_some();
    let mut stale = Vec::new();

    for form in FORMS {
        for addons in combinations(form) {
            let name = snapshot_name(form, &addons);
            let path = snapshots().join(&name);
            let actual = dry_run(form, &addons);
            if bless {
                fs::write(&path, &actual).unwrap();
                continue;
            }
            let expected = fs::read_to_string(&path).unwrap_or_default().replace("\r\n", "\n");
            if expected != actual {
                stale.push(format!("{name}:\n--- snapshot\n{expected}\n--- now\n{actual}"));
            }
        }
    }

    assert!(
        stale.is_empty(),
        "these trees changed - if on purpose, run `LYRN_BLESS=1 cargo test --test trees` and review the diff:\n\n{}",
        stale.join("\n\n")
    );
}

/// A snapshot no combination produces any more would go on looking like a
/// promise about a form that nothing checks.
#[test]
fn every_snapshot_belongs_to_a_combination() {
    let expected: Vec<String> = FORMS
        .iter()
        .flat_map(|form| combinations(form).into_iter().map(move |addons| snapshot_name(form, &addons)))
        .collect();
    for entry in fs::read_dir(snapshots()).unwrap() {
        let name = entry.unwrap().file_name().to_string_lossy().into_owned();
        assert!(expected.contains(&name), "tests/trees/{name} matches no form and add-on set - delete it");
    }
}

/// The snapshot is only worth reading if it can fail: a tree that dropped a
/// file must differ from one that did not.
#[test]
fn a_snapshot_shows_the_add_ons_files() {
    let plain = dry_run("cli", &[]);
    let with_keyring = dry_run("cli", &["keyring"]);
    assert_ne!(plain, with_keyring, "the keyring add-on changed nothing in the tree");
    assert!(with_keyring.contains("secret"), "{with_keyring}");
}
