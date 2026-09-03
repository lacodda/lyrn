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
        "service" => vec![vec![], vec!["spa"]],
        "workspace" => vec![vec![], vec!["keyring"], vec!["self-update"], vec!["keyring", "self-update"]],
        "mono" => vec![vec![], vec!["stand"]],
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
    for form in ["spa", "cli", "desktop", "service", "workspace", "mono"] {
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
    // generated project's CI - and it applies to every Rust form, not just
    // the one where it was first found.
    for (form, roots) in [
        ("cli", vec!["src/main.rs", "src/commands/mod.rs"]),
        ("service", vec!["src/main.rs"]),
        ("desktop", vec!["src-tauri/src/main.rs", "src-tauri/src/lib.rs"]),
        (
            "workspace",
            vec![
                "crates/demo-tool/src/main.rs",
                "crates/demo-tool/src/commands/mod.rs",
                "crates/demo-tool-core/src/lib.rs",
            ],
        ),
    ] {
        for addons in combinations(form) {
            let dir = tempfile::tempdir().unwrap();
            let root = dir.path().join("demo-tool");
            generate(form, &addons, &root);
            check_modules(form, &addons, &root, &roots);
        }
    }
}

/// Every `mod x;` in the named files has a file behind it.
fn check_modules(form: &str, addons: &[&str], root: &Path, sources: &[&str]) {
    for relative in sources {
        let path = root.join(relative);
        let source = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{form}: cannot read {relative}: {e}"));
        let dir_of = path.parent().unwrap().to_path_buf();

        {
            for line in source.lines() {
                let Some(rest) = line.trim().strip_prefix("mod ").or_else(|| line.trim().strip_prefix("pub mod ")) else {
                    continue;
                };
                let Some(module) = rest.strip_suffix(';') else { continue };
                let as_file = dir_of.join(format!("{module}.rs"));
                let as_dir = dir_of.join(module).join("mod.rs");
                assert!(
                    as_file.is_file() || as_dir.is_file(),
                    "{form} with {addons:?}: `mod {module};` in {relative} has no file behind it"
                );
            }
        }
    }
}

#[test]
fn a_workspace_leaves_no_rust_file_outside_its_crates() {
    // A file cargo does not compile is worse than a missing one: it is there
    // to be read and edited, and nothing says the edit had no effect. The
    // workspace form places sources by rule, and a rule that stops applying
    // fails silently - both halves get written, only one gets built.
    for addons in combinations("workspace") {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("demo-tool");
        generate("workspace", &addons, &root);

        let members = members_of(&root.join("Cargo.toml"));
        assert!(!members.is_empty(), "the workspace manifest declares no members");

        for entry in walk(&root) {
            if entry.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let relative = entry.strip_prefix(&root).unwrap().display().to_string().replace('\\', "/");
            assert!(
                members.iter().any(|member| relative.starts_with(&format!("{member}/"))),
                "workspace with {addons:?}: {relative} is Rust that belongs to no member of the workspace"
            );
        }
    }
}

/// The member directories a workspace manifest declares.
fn members_of(manifest: &Path) -> Vec<String> {
    let text = fs::read_to_string(manifest).expect("cannot read the workspace manifest");
    text.lines()
        .find(|l| l.trim_start().starts_with("members"))
        .map(|line| line.split('"').skip(1).step_by(2).map(str::to_string).collect())
        .unwrap_or_default()
}

#[test]
fn a_mono_workspace_has_a_home_for_every_file_it_writes() {
    // A config at the root of a workspace whose tooling is not there is dead
    // weight: nothing reads it, and whoever edits it gets no effect and no
    // warning. So every root config has to name a tool the workspace actually
    // depends on - checked against the dependency list, by exact name, because
    // a substring match would let `vite.config.ts` pass on the strength of
    // `vitest` and wave through exactly the file this exists to catch.
    for addons in combinations("mono") {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("demo-lib");
        generate("mono", &addons, &root);

        let manifest = fs::read_to_string(root.join("package.json")).unwrap();
        let declared: Vec<String> = manifest
            .lines()
            .filter_map(|line| line.trim().strip_prefix('"'))
            .filter_map(|rest| rest.split_once('"'))
            .map(|(key, _)| key.to_string())
            .collect();

        for entry in fs::read_dir(&root).unwrap() {
            let name = entry.unwrap().file_name().to_string_lossy().into_owned();
            if !name.ends_with(".config.ts") && !name.ends_with(".config.js") {
                continue;
            }
            let tool = name.split('.').next().unwrap().to_string();
            assert!(
                declared.contains(&tool),
                "mono with {addons:?}: {name} sits at the root, and `{tool}` is not a dependency of the workspace"
            );
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
