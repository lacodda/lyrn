//! The gate on every form, and on every combination of its add-ons.
//!
//! Generating a project is cheap; the expensive failures are in the seams -
//! a module declared but not written, a section that leaves the file badly
//! formatted, a placeholder that only appears with one add-on switched on.
//! Those only show up when the combination is actually produced, so produce
//! all of them.

use std::fs;
use std::path::Path;

use assert_cmd::Command;

/// The add-on sets each form has to survive.
///
/// Two add-ons is four combinations, which is cheap enough to check
/// exhaustively; if a form ever grows enough of them for that to hurt, the
/// answer is fewer add-ons rather than less checking.
fn combinations(form: &str) -> Vec<Vec<&'static str>> {
    match form {
        "cli" => vec![vec![], vec!["keyring"], vec!["self-update"], vec!["keyring", "self-update"]],
        "desktop" => vec![vec![], vec!["i18n"]],
        _ => vec![vec![]],
    }
}

fn generate(form: &str, addons: &[&str], root: &Path) {
    let mut cmd = Command::cargo_bin("lyrn").unwrap();
    cmd.arg("new")
        .arg("demo-tool")
        .arg("--form")
        .arg(form)
        .arg("--yes")
        .arg("--no-hooks")
        .arg("--repo")
        .arg("owner/demo-tool")
        .arg("--path")
        .arg(root);
    if !addons.is_empty() {
        cmd.arg("--with").arg(addons.join(","));
    }
    cmd.assert().success();
}

#[test]
fn every_combination_renders_completely() {
    for form in ["spa", "cli", "desktop"] {
        for addons in combinations(form) {
            let dir = tempfile::tempdir().unwrap();
            let root = dir.path().join("demo-tool");
            generate(form, &addons, &root);

            // Some files keep their own braces on purpose - a Tera template,
            // a regex matching i18next interpolation. The form says which in
            // its manifest, so read that rather than keeping a second list
            // here that would drift.
            let verbatim = verbatim_of(form);
            for entry in walk(&root) {
                let relative = entry.strip_prefix(&root).unwrap();
                let as_posix = relative.display().to_string().replace('\\', "/");
                if verbatim.contains(&as_posix) {
                    continue;
                }
                // Icons are bytes, not text; reading them as UTF-8 would fail
                // and there is nothing in them to render.
                let Ok(contents) = fs::read_to_string(&entry) else { continue };
                assert!(
                    !contents.contains("{{#") && !contents.contains("{{/"),
                    "{form} with {addons:?}: {} still contains a section marker",
                    relative.display()
                );
                // A GitHub Actions expression is `${{ }}` and stays; a bare
                // `{{ }}` is a placeholder nothing filled in.
                for (index, _) in contents.match_indices("{{") {
                    let is_actions = index > 0 && contents.as_bytes()[index - 1] == b'$';
                    assert!(is_actions, "{form} with {addons:?}: {} has an unrendered placeholder", relative.display());
                }
            }
        }
    }
}

#[test]
fn every_rust_module_a_project_declares_exists() {
    // `mod secrets;` pointing at a file no combination writes compiles only
    // until somebody enables that add-on. Cheaper to catch here than in a
    // generated project's CI.
    for addons in combinations("cli") {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("demo-tool");
        generate("cli", &addons, &root);

        let main = fs::read_to_string(root.join("src/main.rs")).unwrap();
        let commands = fs::read_to_string(root.join("src/commands/mod.rs")).unwrap();

        for (source, dir_of) in [(&main, root.join("src")), (&commands, root.join("src/commands"))] {
            for line in source.lines() {
                let Some(rest) = line.trim().strip_prefix("mod ").or_else(|| line.trim().strip_prefix("pub mod ")) else {
                    continue;
                };
                let Some(module) = rest.strip_suffix(';') else { continue };
                let as_file = dir_of.join(format!("{module}.rs"));
                let as_dir = dir_of.join(module).join("mod.rs");
                assert!(
                    as_file.is_file() || as_dir.is_file(),
                    "cli with {addons:?}: `mod {module};` has no file behind it"
                );
            }
        }
    }
}

#[test]
fn an_unknown_addon_name_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    // A typo in `--with` should stop the run, not silently produce a project
    // missing the thing that was asked for.
    Command::cargo_bin("lyrn")
        .unwrap()
        .args(["new", "demo-tool", "--yes", "--no-hooks", "--with", "nonesuch", "--path"])
        .arg(dir.path().join("demo-tool"))
        .assert()
        .failure();
}

#[test]
fn an_addon_belonging_to_another_form_is_refused() {
    // The harder half: the name is real, so it parses - it just means nothing
    // to this form. Accepting it would succeed and write none of it.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("demo-tool");

    Command::cargo_bin("lyrn")
        .unwrap()
        .args(["new", "demo-tool", "--form", "spa", "--yes", "--no-hooks", "--with", "keyring", "--path"])
        .arg(&root)
        .assert()
        .failure()
        .stderr(predicates::str::contains("has no `keyring` add-on"));

    assert!(!root.exists(), "a refused add-on still created the project");
}

/// The files a form's manifest marks verbatim, as forward-slash paths.
fn verbatim_of(form: &str) -> Vec<String> {
    let manifest = std::fs::read_to_string(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("src/templates/{form}/template.toml")))
        .expect("a form has no manifest");

    manifest
        .lines()
        .skip_while(|l| !l.trim_start().starts_with("verbatim"))
        .take_while(|l| !l.trim().is_empty())
        .flat_map(|l| l.split('"').skip(1).step_by(2))
        .map(str::to_string)
        .collect()
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
