//! The gate on the template: what `lyrn new` actually puts on disk.
//!
//! These run the built binary rather than the library, so they check what a
//! user gets, not what the internals believe they produce.

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use predicates::prelude::*;

mod common;

fn lyrn() -> Command {
    common::lyrn()
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
    // A template that renders half a value produces a project that looks fine
    // and is wrong. `cliff.toml` legitimately carries `{{ }}`: the manifest
    // marks it verbatim, because Tera owns that syntax. A workflow carries
    // `${{ }}`, which is GitHub Actions' own and which the renderer leaves
    // alone - so that, in a workflow and nowhere else, is set aside before
    // looking. Every form: the cli form's install.sh kept
    // `${{{ env_prefix }}_VERSION:-}` for months, because only the spa form
    // was looked at and `${{` was set aside in every file.
    let verbatim = [Path::new("cliff.toml")];

    for form in common::FORMS.iter().filter(|form| **form != "docs") {
        let mut extra = vec!["--form", form];
        if *form == "plugin" {
            extra.extend(["--host", "kilna"]);
        }
        let (_dir, root) = generate("demo-app", &extra);
        for entry in walk(&root) {
            let relative = entry.strip_prefix(&root).unwrap();
            if verbatim.contains(&relative) {
                continue;
            }
            let Ok(contents) = fs::read_to_string(&entry) else { continue };
            let workflow = relative.starts_with(".github/workflows");
            let looked_at = if workflow { contents.replace("${{ ", "") } else { contents };
            assert!(
                !looked_at.contains("{{"),
                "{form}: {} still contains an unrendered placeholder",
                relative.display()
            );
        }
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
    // The standard the templates write, read from them rather than repeated.
    let manifest: toml::Table = toml::from_str(include_str!("../src/templates/spa/template.toml")).unwrap();
    let standard = manifest["standard"].as_str().unwrap();
    assert!(stamp.contains(&format!("standard = \"{standard}\"")), "{stamp}");
    assert!(stamp.contains("with = []"), "{stamp}");
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

/// `--accent <product>` is that product's identity whole: its masters, and every
/// raster drawn from them, before the first commit.
#[test]
fn a_product_of_the_line_brings_its_mark() {
    let (_dir, root) = generate("demo-app", &["--accent", "kilna"]);
    let tile = include_str!("../src/templates/dowel/marks/kilna-S.svg");

    assert_eq!(fs::read_to_string(root.join("assets/logo-s.svg")).unwrap(), tile);
    assert_eq!(
        fs::read_to_string(root.join("public/favicon.svg")).unwrap(),
        tile,
        "the favicon is still the umbrella's"
    );
    assert_eq!(
        fs::read_to_string(root.join("assets/logo.svg")).unwrap(),
        include_str!("../src/templates/dowel/marks/kilna-L.svg")
    );
    assert!(root.join("assets/logo-512.png").is_file(), "the rasters were not drawn");
    assert!(!root.join("assets/banner.svg").exists(), "the banner says the product's name");
    assert!(fs::read_to_string(root.join("lyrn.toml")).unwrap().contains("mark = \"chosen\""));
}

/// A colour colours the palette and draws nothing: the umbrella mark stays on,
/// and lyrn.toml says so.
#[test]
fn a_colour_alone_leaves_the_umbrella_mark_on() {
    let (_dir, root) = generate("demo-app", &["--accent", "#a1b2c3"]);
    assert_eq!(
        fs::read_to_string(root.join("assets/logo-s.svg")).unwrap(),
        include_str!("../src/templates/dowel/marks/lacodda-S.svg")
    );
    assert!(fs::read_to_string(root.join("lyrn.toml")).unwrap().contains("mark = \"placeholder\""));
}

/// A plugin is known by its executable, and a product's casing cannot be read
/// from a slug: `gitlab` is not `Gitlab`, and neither is it GitLab.
#[test]
fn a_plugin_goes_by_its_executable_s_name() {
    let (_dir, root) = generate("gitlab", &["--form", "plugin", "--host", "kasl"]);
    let readme = fs::read_to_string(root.join("README.md")).unwrap();
    assert!(readme.starts_with("# kasl-plugin-gitlab\n"), "{readme}");
    assert!(readme.contains("**Run gitlab**"), "{readme}");
    assert!(readme.contains("beside its own executable"), "kasl's lookup is behind:\n{readme}");
    let contributing = fs::read_to_string(root.join("CONTRIBUTING.md")).unwrap();
    assert!(contributing.starts_with("# Contributing to kasl-plugin-gitlab\n"), "{contributing}");
    for file in ["README.md", "CONTRIBUTING.md", "llms.txt", "src/main.rs"] {
        let text = fs::read_to_string(root.join(file)).unwrap();
        assert!(!text.contains("Gitlab"), "{file} title-cased the slug:\n{text}");
    }
}

/// sefy hands a plugin a sealed file to carry, not a subject to run a command
/// on: the form writes a transport, with the operations sefy reads and none of
/// a command plugin's words.
#[test]
fn a_plugin_for_sefy_is_a_transport() {
    let (_dir, root) = generate("dropbox", &["--form", "plugin", "--host", "sefy", "--repo", "owner/sefy-plugin-dropbox"]);
    let readme = fs::read_to_string(root.join("README.md")).unwrap();
    assert!(readme.starts_with("# sefy-plugin-dropbox\n"), "{readme}");
    assert!(readme.contains("SEFY_DROPBOX_DESTINATION"), "{readme}");
    assert!(readme.contains("\"operations\": [\"push\", \"pull\"]"), "{readme}");

    let main = fs::read_to_string(root.join("src/main.rs")).unwrap();
    assert!(main.contains("\"operations\""), "{main}");
    assert!(!main.contains("\"commands\""), "a transport declared a command:\n{main}");
    assert!(main.contains("const DESTINATION: &str = \"SEFY_DROPBOX_DESTINATION\";"), "{main}");
    let test = fs::read_to_string(root.join("tests/protocol.rs")).unwrap();
    assert!(test.contains("const HOST_OPERATIONS: &[&str] = &[\"push\", \"pull\"];"), "{test}");

    for entry in walk(&root) {
        let relative = entry.strip_prefix(&root).unwrap();
        if relative == Path::new("cliff.toml") || relative.starts_with(".github/workflows") {
            continue;
        }
        let Ok(contents) = fs::read_to_string(&entry) else { continue };
        assert!(
            !contents.contains("{{"),
            "{} has an unrendered placeholder or a section marker",
            relative.display()
        );
    }
}

/// `--with source` writes the rest of kasl's protocol: the requests, the
/// source's own work and the test that asks every request as kasl does.
#[test]
fn a_kasl_plugin_with_source_answers_the_requests() {
    let (_dir, root) = generate(
        "tracker",
        &["--form", "plugin", "--host", "kasl", "--with", "source", "--repo", "owner/kasl-plugin-tracker"],
    );
    for file in ["src/requests.rs", "src/source.rs", "tests/requests.rs"] {
        assert!(root.join(file).is_file(), "{file} was not written");
    }
    let main = fs::read_to_string(root.join("src/main.rs")).unwrap();
    assert!(main.contains("mod requests;") && main.contains("mod source;"), "{main}");
    assert!(main.contains("\"requests\": requests::ANSWERED,"), "{main}");
    assert!(main.contains("\"commands\""), "the source lost the command every plugin has:\n{main}");
    let contributing = fs::read_to_string(root.join("CONTRIBUTING.md")).unwrap();
    assert!(contributing.contains("`src/source.rs`"), "{contributing}");

    let (_plain_dir, plain) = generate("tracker", &["--form", "plugin", "--host", "kasl", "--repo", "owner/kasl-plugin-tracker"]);
    assert!(!plain.join("src/requests.rs").exists(), "the requests came without the add-on");
    assert!(!fs::read_to_string(plain.join("src/main.rs")).unwrap().contains("requests"));
}

/// The add-on is part of one host's protocol: asking for it with another host
/// is refused with who takes it, rather than writing code nothing calls.
#[test]
fn source_is_refused_for_a_host_that_asks_no_requests() {
    for host in ["kilna", "sefy"] {
        let dir = tempfile::tempdir().unwrap();
        let output = lyrn()
            .args(["new", "demo", "--yes", "--no-hooks", "--form", "plugin", "--host", host, "--with", "source"])
            .arg("--path")
            .arg(dir.path().join("demo"))
            .assert()
            .failure()
            .get_output()
            .stderr
            .clone();
        let stderr = String::from_utf8_lossy(&output);
        assert!(stderr.contains("a kasl plugin"), "{host}: {stderr}");
        assert!(!dir.path().join("demo").exists(), "{host}: a refused project was written");
    }
}

/// A block is one of dowel's screens, written with every primitive it is made
/// of and nothing else: `--with block:list-detail` is `lyrn add
/// list-detail-screen` at the moment the project is made, from the copies the
/// binary carries.
#[test]
fn a_block_comes_with_the_primitives_it_is_made_of() {
    let (_dir, root) = generate("demo-app", &["--with", "block:list-detail,router"]);
    let block = fs::read_to_string(root.join("src/components/blocks/list-detail-screen.tsx")).unwrap();
    for import in block.lines().filter_map(|l| l.split("from '@/components/ui/").nth(1)) {
        let part = import.trim_end_matches(['\'', ';']);
        assert!(
            root.join(format!("src/components/ui/{part}.tsx")).is_file(),
            "the block imports {part}, and it is not there"
        );
    }
    // What the parts import of each other comes too: frame is made of a scroll area.
    assert!(root.join("src/components/ui/scroll-area.tsx").is_file());
    // A block is not an add-on of the form: `lyrn upgrade` renders the
    // standard by `with`, and a block changes nothing in it.
    let lyrn_toml = fs::read_to_string(root.join("lyrn.toml")).unwrap();
    assert!(lyrn_toml.contains("with = [\"router\"]"), "{lyrn_toml}");
    // Nothing of another block.
    assert!(!root.join("src/components/blocks/login-screen.tsx").exists());
}

/// A service draws with dowel only through its web UI, and the block goes
/// where that UI's components.json says.
#[test]
fn a_service_s_block_goes_into_its_frontend() {
    let (_dir, root) = generate("demo-svc", &["--form", "service", "--with", "spa,block:settings"]);
    assert!(root.join("frontend/src/components/blocks/settings-screen.tsx").is_file());
    assert!(root.join("frontend/src/components/ui/section-nav.tsx").is_file());
}

#[test]
fn a_block_is_refused_where_nothing_draws_with_dowel_and_by_a_name_dowel_lacks() {
    for (args, says) in [
        (vec!["--form", "cli", "--with", "block:login"], "the `cli` form draws nothing of dowel's"),
        (vec!["--form", "service", "--with", "block:login"], "`--with spa` as well"),
        (vec!["--with", "block:nothing"], "has no `nothing` block - it has: block:"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let output = lyrn()
            .args(["new", "demo", "--yes", "--no-hooks"])
            .args(&args)
            .arg("--path")
            .arg(dir.path().join("demo"))
            .assert()
            .failure()
            .get_output()
            .stderr
            .clone();
        let stderr = String::from_utf8_lossy(&output);
        assert!(stderr.contains(says), "{args:?}: {stderr}");
        assert!(!dir.path().join("demo").exists(), "{args:?}: a refused project was written");
    }
}

/// A stand-in rigger whose profile keeps the line's repositories in `root`,
/// which knows no project yet, and which writes down every call it gets.
struct Record {
    dir: tempfile::TempDir,
}

impl Record {
    fn new(root: &Path, hubs: &Path) -> Record {
        let dir = tempfile::tempdir().unwrap();
        let slashed = |p: &Path| p.display().to_string().replace(std::path::MAIN_SEPARATOR, "/");
        fs::write(
            dir.path().join("profile.json"),
            format!(
                "[{{\"current\":true,\"name\":\"line\",\"profile\":{{\"hubs\":\"{}\",\"kind\":\"line\",\"roots\":[\"{}\"]}}}}]\n",
                slashed(hubs),
                slashed(root)
            ),
        )
        .unwrap();
        fs::write(dir.path().join("projects.json"), "[]\n").unwrap();
        if cfg!(windows) {
            fs::write(
                dir.path().join("rigger.cmd"),
                "@echo off\r\necho %* >> \"%~dp0calls.txt\"\r\nif \"%2\"==\"profile\" type \"%~dp0profile.json\" & exit /b 0\r\nif \"%2\"==\"project\" type \"%~dp0projects.json\" & exit /b 0\r\nexit /b 0\r\n",
            )
            .unwrap();
        } else {
            let script = dir.path().join("rigger");
            fs::write(
                &script,
                "#!/bin/sh\nhere=$(dirname \"$0\")\necho \"$*\" >> \"$here/calls.txt\"\ncase \"$2\" in\n  profile) cat \"$here/profile.json\" ;;\n  project) cat \"$here/projects.json\" ;;\nesac\nexit 0\n",
            )
            .unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
            }
            // An executable just written can be "busy" on Linux while another
            // test's fork still holds it open: run it until it runs, so the
            // generation never meets that flake - and only that one.
            for _ in 0..50 {
                match std::process::Command::new(&script).arg("--version").output() {
                    Err(e) if e.raw_os_error() == Some(26) => std::thread::sleep(std::time::Duration::from_millis(50)),
                    _ => break,
                }
            }
        }
        Record { dir }
    }

    fn program(&self) -> std::path::PathBuf {
        self.dir.path().join(if cfg!(windows) { "rigger.cmd" } else { "rigger" })
    }

    fn calls(&self) -> String {
        fs::read_to_string(self.dir.path().join("calls.txt")).unwrap_or_default()
    }
}

/// The cli form written out as a template, with the hooks that start the
/// repository and without the one that fetches crates: a generation that runs
/// its hooks, and needs no network.
fn template_without_a_fetch(at: &Path) -> std::path::PathBuf {
    let template = at.join("template-cli");
    lyrn()
        .args(["template", "export", "cli", "--repo", "owner/template-cli", "--path"])
        .arg(&template)
        .assert()
        .success();
    let manifest = fs::read_to_string(template.join("template.toml")).unwrap();
    let kept: Vec<&str> = manifest.split("[[hooks]]").filter(|part| !part.contains("\"cargo\", \"fetch\"")).collect();
    fs::write(template.join("template.toml"), kept.join("[[hooks]]")).unwrap();
    template
}

/// The project starts its life in the record: recorded, its shape, its hub
/// written out of the record, its skill - the moment the repository exists.
#[test]
fn a_repository_made_where_the_line_lives_joins_the_record() {
    let line = tempfile::tempdir().unwrap();
    let hubs = tempfile::tempdir().unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    let template = template_without_a_fetch(elsewhere.path());
    let record = Record::new(line.path(), hubs.path());
    let root = line.path().join("demo-tool");
    let output = lyrn()
        .env("LYRN_RIGGER", record.program())
        .env("GIT_AUTHOR_NAME", "Ada Lovelace")
        .env("GIT_AUTHOR_EMAIL", "ada@example.com")
        .env("GIT_COMMITTER_NAME", "Ada Lovelace")
        .env("GIT_COMMITTER_EMAIL", "ada@example.com")
        .args(["new", "demo-tool", "--yes", "--repo", "owner/demo-tool", "--template"])
        .arg(&template)
        .arg("--path")
        .arg(&root)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8_lossy(&output);
    assert!(stdout.contains("Recorded in rigger as demo-tool, its hub at"), "{stdout}");
    let calls = record.calls();
    for call in [
        "project add",
        "--name demo-tool",
        "project mark demo-tool --form cli",
        "export demo-tool --hub",
        "skill demo-tool --install",
    ] {
        assert!(calls.contains(call), "no `{call}` in:\n{calls}");
    }
    assert!(hubs.path().join("demo-tool").is_dir(), "the hub's directory was not made");

    // Made in a scratch directory instead: the record is not told.
    let scratch = tempfile::tempdir().unwrap();
    let record = Record::new(line.path(), hubs.path());
    let output = lyrn()
        .env("LYRN_RIGGER", record.program())
        .env("GIT_AUTHOR_NAME", "Ada Lovelace")
        .env("GIT_AUTHOR_EMAIL", "ada@example.com")
        .env("GIT_COMMITTER_NAME", "Ada Lovelace")
        .env("GIT_COMMITTER_EMAIL", "ada@example.com")
        .args(["new", "demo-tool", "--yes", "--repo", "owner/demo-tool", "--template"])
        .arg(&template)
        .arg("--path")
        .arg(scratch.path().join("demo-tool"))
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8_lossy(&output);
    assert!(stdout.contains("Not recorded in rigger: the line's repositories are in"), "{stdout}");
    assert!(!record.calls().contains("project add"), "{}", record.calls());
}
