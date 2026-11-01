//! The release contour every form writes, held to what furca's release
//! engine reads and to the order a release goes out in.
//!
//! A release is the one moment a project's files are read by machines that
//! are not its CI - the engine, the forge, the registries, a stranger's
//! installer - and a mistake in them shows only then, on a tag that cannot
//! be taken back. So each form's contour is generated here and read the way
//! those machines read it.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use yaml_rust2::{Yaml, YamlLoader};

mod common;

/// The forms that start a repository: the docs form adds to one.
fn forms() -> impl Iterator<Item = &'static str> {
    common::FORMS.iter().copied().filter(|form| *form != "docs")
}

/// Generate `form` with `addons` into a fresh directory.
fn generate(form: &str, addons: &[&str]) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("demo-tool");
    let mut cmd = common::lyrn();
    cmd.args(["new", "demo-tool", "--yes", "--no-hooks", "--repo", "owner/demo-tool", "--form", form])
        .arg("--path")
        .arg(&root);
    if form == "plugin" {
        cmd.args(["--host", "kilna"]);
    }
    if !addons.is_empty() {
        cmd.arg("--with").arg(addons.join(","));
    }
    cmd.assert().success();
    (dir, root)
}

/// The generated tree as its first commit: the engine reads a work tree, and
/// holds only the files git tracks.
fn commit(root: &Path) {
    for args in [
        &["init", "--quiet", "--initial-branch=main"][..],
        &["add", "."],
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.com",
            "commit",
            "--quiet",
            "-m",
            "feat: start",
        ],
    ] {
        let status = Command::new("git").args(args).current_dir(root).status().unwrap();
        assert!(status.success(), "git {args:?} in {}", root.display());
    }
}

fn workflow(root: &Path, name: &str) -> Option<Yaml> {
    let text = fs::read_to_string(root.join(".github/workflows").join(name)).ok()?;
    Some(YamlLoader::load_from_str(&text).unwrap().remove(0))
}

/// The jobs of a workflow, by id.
fn jobs(workflow: &Yaml) -> Vec<(String, &Yaml)> {
    workflow["jobs"]
        .as_hash()
        .expect("a workflow has jobs")
        .iter()
        .map(|(id, job)| (id.as_str().unwrap().to_string(), job))
        .collect()
}

fn needs(job: &Yaml) -> Vec<String> {
    match &job["needs"] {
        Yaml::String(one) => vec![one.clone()],
        Yaml::Array(many) => many.iter().filter_map(|n| n.as_str().map(String::from)).collect(),
        _ => Vec::new(),
    }
}

/// Every form writes a release.toml that furca's engine reads without a
/// problem, with every add-on: a key it does not know, a manifest or a
/// workflow that is not there, would stop the first release of a project
/// that was never released by hand.
#[test]
fn every_form_writes_a_release_toml_furca_reads() {
    for form in forms() {
        for addons in common::combinations(form) {
            let (_dir, root) = generate(form, &addons);
            if let Err(error) = furca_release::Config::load(&root) {
                panic!("{form} {addons:?}: furca does not read release.toml: {error}");
            }
        }
    }
}

/// The manifests release.toml names are every file of the project that says
/// its version: one left out keeps the old number after the bump, and the
/// shopfronts disagree from the first release on.
#[test]
fn release_toml_names_every_manifest_that_carries_the_version() {
    for form in forms() {
        for addons in common::combinations(form) {
            let (_dir, root) = generate(form, &addons);
            let config = furca_release::Config::load(&root).unwrap();
            let named: Vec<&str> = config.project.manifests.iter().map(String::as_str).collect();
            for manifest in ["Cargo.toml", "package.json", "tauri.conf.json"] {
                for found in files_named(&root, manifest) {
                    let text = fs::read_to_string(root.join(&found)).unwrap();
                    let versioned = if manifest == "Cargo.toml" {
                        let table: toml::Table = toml::from_str(&text).unwrap();
                        let in_package = table.get("package").and_then(|p| p.get("version")).and_then(|v| v.as_str()).is_some();
                        let in_workspace = table.get("workspace").and_then(|w| w.get("package")).and_then(|p| p.get("version")).is_some();
                        in_package || in_workspace
                    } else {
                        // A private package at 0.0.0 - mono's stand - says it
                        // has no version of its own and never ships.
                        let json: serde_json::Value = serde_json::from_str(&text).unwrap();
                        let placeholder = json["private"] == true && json["version"] == "0.0.0";
                        json.get("version").is_some() && !placeholder
                    };
                    // A member of a workspace takes its version from the root,
                    // which the root's entry covers.
                    let inherits = text.contains("version.workspace = true");
                    if versioned && !inherits {
                        assert!(
                            named.contains(&found.as_str()),
                            "{form} {addons:?}: {found} carries the version and release.toml does not name it"
                        );
                    }
                }
            }
        }
    }
}

/// Every manifest file under `root`, outside dependencies and builds, as a
/// `/`-separated path.
fn files_named(root: &Path, name: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            let file = path.file_name().unwrap().to_string_lossy().into_owned();
            if path.is_dir() {
                if !matches!(file.as_str(), "node_modules" | "target" | ".git" | "dist") {
                    stack.push(path);
                }
            } else if file == name {
                found.push(path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/"));
            }
        }
    }
    found
}

/// Nothing goes to a registry before the release is out: an npm wrapper
/// downloads its binary from the release page, and published first it
/// answers `npm install` with a 404 until the archives arrive - lyrn 2.10.0
/// did, on the CI of its templates. Every job of every publish workflow,
/// lyrn's own included, needs the one that waits for release.yml, and that
/// one waits for the workflow the repository actually has.
#[test]
fn every_publish_waits_for_the_release() {
    let mut held = 0;
    let mut roots: Vec<(String, tempfile::TempDir, PathBuf)> = Vec::new();
    for form in forms() {
        let (dir, root) = generate(form, &[]);
        roots.push((form.to_string(), dir, root));
    }
    let own = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
    let mut trees: Vec<(String, PathBuf)> = roots.iter().map(|(f, _, r)| (f.clone(), r.clone())).collect();
    trees.push(("lyrn".to_string(), own));

    for (name, root) in &trees {
        let Some(publish) = workflow(root, "publish.yml") else { continue };
        assert!(
            root.join(".github/workflows/release.yml").is_file(),
            "{name}: publish.yml waits for a release.yml that is not there"
        );
        let jobs = jobs(&publish);
        let (_, wait) = jobs
            .iter()
            .find(|(id, _)| id == "release")
            .unwrap_or_else(|| panic!("{name}: publish.yml has no `release` job to wait with"));
        let script: String = wait["steps"].as_vec().unwrap().iter().filter_map(|s| s["run"].as_str()).collect();
        assert!(
            script.contains("--workflow release.yml"),
            "{name}: the `release` job does not wait for release.yml"
        );
        assert!(
            script.contains("completed/success"),
            "{name}: the `release` job does not insist on a green release"
        );
        for (id, job) in jobs.iter().filter(|(id, _)| id != "release") {
            assert!(
                needs(job).contains(&"release".to_string()),
                "{name}: the `{id}` job of publish.yml does not wait for the release"
            );
        }
        held += 1;
    }
    assert!(held >= 5, "only {held} publish workflows were looked at");
}

/// Every form's release page ends with the `Checked live` list: the notes are
/// the changelog's section and then .github/release-notes.md, and the list
/// has a box for how the release reaches a machine and one for running it by
/// hand.
#[test]
fn every_release_page_ends_with_the_checklist() {
    let own = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
    let mut trees: Vec<(String, Option<tempfile::TempDir>, PathBuf)> = forms()
        .map(|form| {
            let (dir, root) = generate(form, &[]);
            (form.to_string(), Some(dir), root)
        })
        .collect();
    trees.push(("lyrn".to_string(), None, own));

    for (name, _dir, root) in &trees {
        let release = workflow(root, "release.yml").unwrap_or_else(|| panic!("{name}: no release.yml writes the release page"));
        let steps: Vec<&Yaml> = jobs(&release).iter().flat_map(|(_, job)| job["steps"].as_vec().unwrap().iter()).collect();
        let writes = steps
            .iter()
            .filter_map(|s| s["run"].as_str())
            .any(|run| run.contains("cat .github/release-notes.md >> release-notes.md"));
        assert!(writes, "{name}: release.yml does not end the notes with .github/release-notes.md");
        let page = steps
            .iter()
            .find(|s| s["uses"].as_str().is_some_and(|u| u.starts_with("softprops/action-gh-release")))
            .unwrap_or_else(|| panic!("{name}: release.yml creates no release"));
        assert_eq!(
            page["with"]["body_path"].as_str(),
            Some("release-notes.md"),
            "{name}: the release is not written from the notes"
        );

        let notes = fs::read_to_string(root.join(".github/release-notes.md")).unwrap();
        assert!(notes.starts_with("### Checked live\n"), "{name}: {notes}");
        let boxes = notes.lines().filter(|l| l.starts_with("- [ ] ")).count();
        assert!(boxes >= 2, "{name}: the checklist has {boxes} boxes:\n{notes}");
        assert!(notes.contains("Run by hand"), "{name}: {notes}");
    }
}

/// The engine holds every target an installer asks for to the files the
/// release page will carry, and `lyrn check` reports it: kasl's installer
/// once asked for archives no release had ever built.
#[test]
fn the_installers_ask_for_what_the_release_page_carries() {
    let (_dir, root) = generate("cli", &[]);
    commit(&root);
    // `commands` reads the program's `--help`: a stand-in, newer than the
    // sources, spares the check a build. What it says is not looked at here.
    let program = root.join("target/debug").join(format!("demo-tool{}", std::env::consts::EXE_SUFFIX));
    fs::create_dir_all(program.parent().unwrap()).unwrap();
    fs::copy(env!("CARGO_BIN_EXE_lyrn"), &program).unwrap();
    fs::File::options()
        .write(true)
        .open(&program)
        .unwrap()
        .set_modified(std::time::SystemTime::now())
        .unwrap();
    let run = |root: &Path| {
        let out = common::lyrn().arg("check").arg(root).env_remove("CARGO_TARGET_DIR").output().unwrap();
        String::from_utf8(out.stdout).unwrap()
    };
    let report = run(&root);
    let installers = report.lines().find(|l| l.contains(" installers ")).unwrap_or_else(|| panic!("{report}"));
    assert!(
        installers.starts_with('✓') && installers.contains("for files the release page carries"),
        "{report}"
    );
    assert!(report.lines().any(|l| l.starts_with("✓ release ")), "{report}");

    let sh = fs::read_to_string(root.join("tools/install.sh")).unwrap();
    fs::write(
        root.join("tools/install.sh"),
        sh.replace("x86_64-unknown-linux-gnu", "x86_64-unknown-linux-musl"),
    )
    .unwrap();
    let report = run(&root);
    assert!(report.contains("✗ installers"), "{report}");
    assert!(report.contains("asks for x86_64-unknown-linux-musl"), "{report}");
}

/// The installers a form writes are scripts a shell accepts: the cli form's
/// install.sh shipped `${{{ env_prefix }}_VERSION:-}` unrendered, a bad
/// substitution sh refuses before it runs a line.
#[cfg(unix)]
#[test]
fn the_unix_installer_is_a_script_sh_accepts() {
    for form in ["cli", "workspace"] {
        let (_dir, root) = generate(form, &[]);
        let out = Command::new("sh").arg("-n").arg(root.join("tools/install.sh")).output().unwrap();
        assert!(out.status.success(), "{form}: sh refuses install.sh: {}", String::from_utf8_lossy(&out.stderr));
        let text = fs::read_to_string(root.join("tools/install.sh")).unwrap();
        assert!(text.contains("${DEMO_TOOL_VERSION:-}"), "{form}: the pinned version is not read:\n{text}");
    }
}
