//! `lyrn adopt`: an existing repository brought up to the line's standard.

use std::fs;
use std::path::Path;

use assert_cmd::Command;

mod common;

fn adopt(dir: &Path) -> Command {
    let mut cmd = common::lyrn();
    cmd.args(["adopt", "--yes", "--author", "Tester", "--repo", "owner/old-tool"]).arg(dir);
    cmd
}

fn write(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// A command-line tool from before lyrn: a manifest, a main, a README and a
/// licence under a name of its own.
fn old_cli(root: &Path) {
    write(
        root,
        "Cargo.toml",
        "[package]\nname = \"old-tool\"\nversion = \"0.3.0\"\ndescription = \"An old tool\"\nedition = \"2024\"\n",
    );
    write(root, "src/main.rs", "fn main() {}\n");
    write(root, "README.md", "# old-tool, as it was\n");
    write(root, "LICENSE.md", "MIT, as it was\n");
}

#[test]
fn only_what_is_missing_is_added_and_nothing_is_replaced() {
    let dir = tempfile::tempdir().unwrap();
    old_cli(dir.path());

    adopt(dir.path()).assert().success();

    for added in [
        "CHANGELOG.md",
        "cliff.toml",
        ".editorconfig",
        ".gitattributes",
        ".gitignore",
        ".github/workflows/ci.yml",
        "lyrn.toml",
    ] {
        assert!(dir.path().join(added).is_file(), "{added} was not added");
    }
    assert_eq!(fs::read_to_string(dir.path().join("README.md")).unwrap(), "# old-tool, as it was\n");
    // `LICENSE.md` is the licence; a second one beside it would be two.
    assert!(!dir.path().join("LICENSE").exists(), "a LICENSE was added beside LICENSE.md");
    assert_eq!(fs::read_to_string(dir.path().join("src/main.rs")).unwrap(), "fn main() {}\n");
}

/// What the repository says about itself is what `lyrn.toml` records: the
/// form read from its files, the name from its manifest.
#[test]
fn lyrn_toml_records_what_the_repository_is() {
    let dir = tempfile::tempdir().unwrap();
    old_cli(dir.path());
    adopt(dir.path()).assert().success();

    let lyrn_toml = fs::read_to_string(dir.path().join("lyrn.toml")).unwrap();
    assert!(lyrn_toml.contains("name = \"old-tool\""), "{lyrn_toml}");
    assert!(lyrn_toml.contains("form = \"cli\""), "{lyrn_toml}");
    assert!(lyrn_toml.contains("mark = \"placeholder\""), "{lyrn_toml}");
}

#[test]
fn a_second_run_has_nothing_to_add() {
    let dir = tempfile::tempdir().unwrap();
    old_cli(dir.path());
    adopt(dir.path()).assert().success();

    let out = adopt(dir.path()).assert().success().get_output().stdout.clone();
    assert!(String::from_utf8_lossy(&out).contains("Nothing to add"));
}

#[test]
fn a_dry_run_says_what_is_kept_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    old_cli(dir.path());

    let out = adopt(dir.path()).arg("--dry-run").assert().success().get_output().stdout.clone();
    let text = String::from_utf8_lossy(&out);
    assert!(text.contains("README.md  (already there)"), "{text}");
    assert!(text.contains("LICENSE  (`LICENSE.md` is there)"), "{text}");
    assert!(!dir.path().join("lyrn.toml").exists(), "--dry-run wrote lyrn.toml");
}

/// A product of the line, adopted, wears its own colour without being told.
#[test]
fn a_product_of_the_line_is_given_its_accent() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "Cargo.toml",
        "[package]\nname = \"hilvan\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    write(dir.path(), "src/main.rs", "fn main() {}\n");
    adopt(dir.path()).assert().success();

    let lyrn_toml = fs::read_to_string(dir.path().join("lyrn.toml")).unwrap();
    assert!(lyrn_toml.contains("accent = \"#D64550\""), "{lyrn_toml}");
    assert!(lyrn_toml.contains("mark = \"chosen\""), "{lyrn_toml}");
}

#[test]
fn the_form_is_read_from_what_the_repository_holds() {
    let cases: &[(&str, &[(&str, &str)])] = &[
        ("spa", &[("package.json", r#"{"name":"web","devDependencies":{"vite":"^8"}}"#)]),
        (
            "desktop",
            &[
                ("package.json", r#"{"name":"app","devDependencies":{"vite":"^8"}}"#),
                ("src-tauri/tauri.conf.json", "{}"),
            ],
        ),
        (
            "service",
            &[
                ("Cargo.toml", "[package]\nname = \"svc\"\n[dependencies]\naxum = \"0.8\"\n"),
                ("src/main.rs", "fn main() {}"),
            ],
        ),
        ("workspace", &[("Cargo.toml", "[workspace]\nmembers = [\"crates/a\"]\n")]),
        (
            "mono",
            &[
                ("package.json", r#"{"name":"lib-root","private":true}"#),
                ("pnpm-workspace.yaml", "packages:\n  - packages/*\n"),
                ("packages/lib/package.json", r#"{"name":"lib"}"#),
            ],
        ),
        (
            "plugin",
            &[("Cargo.toml", "[package]\nname = \"kilna-plugin-count\"\n"), ("src/main.rs", "fn main() {}")],
        ),
    ];
    for (form, files) in cases {
        let dir = tempfile::tempdir().unwrap();
        for (path, text) in *files {
            write(dir.path(), path, text);
        }
        adopt(dir.path()).args(["--name", "demo-tool"]).assert().success();
        let lyrn_toml = fs::read_to_string(dir.path().join("lyrn.toml")).unwrap();
        assert!(lyrn_toml.contains(&format!("form = \"{form}\"")), "expected {form}:\n{lyrn_toml}");
    }
}

/// A guess is worse than a question: a directory lyrn cannot read is refused
/// with the forms it could be, rather than adopted as the default.
#[test]
fn a_repository_of_no_known_form_is_asked_about() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "notes.txt", "just notes");
    let out = adopt(dir.path()).assert().failure().get_output().stderr.clone();
    let text = String::from_utf8_lossy(&out);
    assert!(text.contains("--form") && text.contains("cli"), "{text}");
    assert!(!dir.path().join("lyrn.toml").exists());
}

#[test]
fn a_library_crate_is_not_taken_for_a_command_line_tool() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "Cargo.toml", "[package]\nname = \"old-lib\"\n");
    write(dir.path(), "src/lib.rs", "pub fn f() {}\n");
    adopt(dir.path()).assert().failure();
    adopt(dir.path()).args(["--form", "cli"]).assert().success();
}

#[test]
fn what_lyrn_toml_says_wins_over_what_the_files_suggest() {
    let dir = tempfile::tempdir().unwrap();
    // A service by its dependencies, a workspace by its own word.
    write(dir.path(), "Cargo.toml", "[package]\nname = \"svc\"\n[dependencies]\naxum = \"0.8\"\n");
    write(dir.path(), "lyrn.toml", "[project]\nname = \"svc\"\nform = \"workspace\"\n");
    let out = adopt(dir.path()).arg("--dry-run").assert().success().get_output().stdout.clone();
    assert!(String::from_utf8_lossy(&out).contains("as a `workspace` project"));
}

#[test]
fn a_documentation_site_is_not_something_a_repository_is() {
    let dir = tempfile::tempdir().unwrap();
    old_cli(dir.path());
    let out = adopt(dir.path()).args(["--form", "docs"]).assert().failure().get_output().stderr.clone();
    assert!(String::from_utf8_lossy(&out).contains("--form docs"));
}

/// Decisions the repository already keeps keep their numbers, and nothing is
/// slotted in among them: a second 0001 or 0002 would make "ADR 1" mean two
/// things, and an index written for the standard's decisions would list these
/// wrongly.
#[test]
fn decisions_of_its_own_are_left_as_they_are() {
    let dir = tempfile::tempdir().unwrap();
    old_cli(dir.path());
    write(dir.path(), "docs/adr/0001-use-rust.md", "# Use Rust\n");
    adopt(dir.path()).assert().success();
    for added in ["0001-record-architecture-decisions.md", "README.md"] {
        assert!(
            !dir.path().join("docs/adr").join(added).exists(),
            "adopt added docs/adr/{added} among the repository's own"
        );
    }
    assert_eq!(std::fs::read_to_string(dir.path().join("docs/adr/0001-use-rust.md")).unwrap(), "# Use Rust\n");
}

/// A repository with no decisions recorded gets the standard's sequence whole:
/// both decisions and the index that lists them, so no link in it dangles.
#[test]
fn a_repository_without_decisions_gets_the_whole_sequence() {
    let dir = tempfile::tempdir().unwrap();
    old_cli(dir.path());
    adopt(dir.path()).assert().success();
    let index = std::fs::read_to_string(dir.path().join("docs/adr/README.md")).unwrap();
    for decision in ["0001-record-architecture-decisions.md", "0002-dependabot-is-off.md"] {
        assert!(dir.path().join("docs/adr").join(decision).is_file(), "docs/adr/{decision} was not added");
        assert!(index.contains(decision), "the index does not list {decision}");
    }
}

/// Cargo's other place for binaries: a crate with `src/bin/` is a program,
/// whatever its `src/lib.rs` says.
#[test]
fn binaries_under_src_bin_make_a_program() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "Cargo.toml", "[package]\nname = \"viewer\"\n");
    write(dir.path(), "src/lib.rs", "pub fn f() {}\n");
    write(dir.path(), "src/bin/viewer.rs", "fn main() {}\n");
    let out = adopt(dir.path()).arg("--dry-run").assert().success().get_output().stdout.clone();
    assert!(String::from_utf8_lossy(&out).contains("as a `cli` project"));
}

/// A dependency declared for one platform is still a dependency.
#[test]
fn a_platform_dependency_decides_the_form_too() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "Cargo.toml",
        "[package]\nname = \"svc\"\n[target.'cfg(unix)'.dependencies]\naxum = \"0.8\"\n",
    );
    write(dir.path(), "src/main.rs", "fn main() {}\n");
    let out = adopt(dir.path()).arg("--dry-run").assert().success().get_output().stdout.clone();
    assert!(String::from_utf8_lossy(&out).contains("as a `service` project"));
}

/// A log of its own stays whole, and the decision the audit points at joins it
/// at the next free number: a link to the standard's 0002 would land on another
/// decision, or on nothing.
#[test]
fn the_audit_points_at_the_next_free_decision_in_a_log_of_its_own() {
    let dir = tempfile::tempdir().unwrap();
    old_cli(dir.path());
    write(dir.path(), "docs/adr/0001-use-rust.md", "# Use Rust\n");
    write(dir.path(), "docs/adr/0002-store-in-sqlite.md", "# Store in SQLite\n");
    write(dir.path(), "docs/adr/README.md", "# Decisions\n");
    adopt(dir.path()).assert().success();

    let decision = dir.path().join("docs/adr/0003-dependabot-is-off.md");
    let text = fs::read_to_string(&decision).unwrap();
    assert!(text.starts_with("# ADR 0003:"), "{text}");
    for naming in [".github/workflows/audit.yml", "deny.toml"] {
        let text = fs::read_to_string(dir.path().join(naming)).unwrap();
        assert!(
            text.contains("docs/adr/0003-dependabot-is-off.md"),
            "{naming} does not name the decision:\n{text}"
        );
        assert!(!text.contains("0002-dependabot-is-off"), "{naming} still names the standard's number");
    }
    // The repository's own index is not rewritten.
    assert_eq!(fs::read_to_string(dir.path().join("docs/adr/README.md")).unwrap(), "# Decisions\n");
}

/// A decision it already keeps, under any number, is the one the audit names.
#[test]
fn a_decision_already_kept_is_named_by_its_own_number() {
    let dir = tempfile::tempdir().unwrap();
    old_cli(dir.path());
    write(dir.path(), "docs/adr/0007-dependabot-is-off.md", "# Off\n");
    write(dir.path(), "docs/adr/0009-later.md", "# Later\n");
    adopt(dir.path()).assert().success();

    assert!(!dir.path().join("docs/adr/0010-dependabot-is-off.md").exists());
    let audit = fs::read_to_string(dir.path().join(".github/workflows/audit.yml")).unwrap();
    assert!(audit.contains("docs/adr/0007-dependabot-is-off.md"), "{audit}");
}

/// A desktop app: the licence tool audit.yml runs is written with it.
fn old_desktop(root: &Path) {
    write(
        root,
        "package.json",
        "{\"name\": \"window\", \"description\": \"A window\", \"devDependencies\": {\"vite\": \"8\"}}\n",
    );
    write(root, "src-tauri/Cargo.toml", "[package]\nname = \"window\"\n");
    write(root, "src-tauri/tauri.conf.json", "{}\n");
}

#[test]
fn the_licence_tool_the_audit_runs_is_added_with_it() {
    let dir = tempfile::tempdir().unwrap();
    old_desktop(dir.path());
    adopt(dir.path()).assert().success();

    let audit = fs::read_to_string(dir.path().join(".github/workflows/audit.yml")).unwrap();
    assert!(audit.contains("node tools/check-licenses.mjs"), "{audit}");
    let tool = fs::read_to_string(dir.path().join("tools/check-licenses.mjs")).unwrap();
    assert!(tool.contains("ONE_BY_ONE"), "the tool cannot accept a package by name:\n{tool}");
}

/// In a workspace the Cargo.lock is at the top, and the audit has to read it
/// there; a lone window is audited where it is.
#[test]
fn a_desktop_app_in_a_workspace_is_audited_from_the_workspace() {
    let lone = tempfile::tempdir().unwrap();
    old_desktop(lone.path());
    adopt(lone.path()).assert().success();
    let audit = fs::read_to_string(lone.path().join(".github/workflows/audit.yml")).unwrap();
    assert!(audit.contains("manifest-path: src-tauri/Cargo.toml"), "{audit}");

    let workspace = tempfile::tempdir().unwrap();
    old_desktop(workspace.path());
    write(workspace.path(), "Cargo.toml", "[workspace]\nmembers = [\"src-tauri\", \"crates/*\"]\n");
    adopt(workspace.path()).args(["--form", "desktop"]).assert().success();
    let audit = fs::read_to_string(workspace.path().join(".github/workflows/audit.yml")).unwrap();
    assert!(audit.contains("EmbarkStudios/cargo-deny-action"), "{audit}");
    assert!(!audit.contains("manifest-path"), "{audit}");
}

/// A service with its web UI and its site in places of its own: every lockfile
/// gets an audit, and the next steps do not claim the site is missing.
#[test]
fn every_lockfile_gets_an_audit_and_a_site_is_seen_wherever_it_is() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "Cargo.toml", "[package]\nname = \"svc\"\n[dependencies]\naxum = \"0.8\"\n");
    write(dir.path(), "src/main.rs", "fn main() {}\n");
    write(dir.path(), "Cargo.lock", "version = 4\n");
    write(dir.path(), "web/package.json", "{}\n");
    write(dir.path(), "web/pnpm-lock.yaml", "lockfileVersion: '9.0'\n");
    write(dir.path(), "docs/site/package.json", "{}\n");
    write(dir.path(), "docs/site/pnpm-lock.yaml", "lockfileVersion: '9.0'\n");
    write(dir.path(), "docs/site/astro.config.mjs", "export default {}\n");

    let out = adopt(dir.path()).assert().success().get_output().stdout.clone();
    let out = String::from_utf8_lossy(&out);
    assert!(!out.contains("--form docs"), "the site in docs/site/ was not seen:\n{out}");

    let audit = fs::read_to_string(dir.path().join(".github/workflows/audit.yml")).unwrap();
    assert!(audit.contains("packages-web:"), "{audit}");
    assert!(audit.contains("working-directory: web"), "{audit}");
    assert!(audit.contains("package_json_file: web/package.json"), "{audit}");
    assert!(
        dir.path().join("web/tools/check-licenses.mjs").is_file(),
        "the job's licence tool was not added"
    );

    let site = fs::read_to_string(dir.path().join(".github/workflows/docs-audit.yml")).unwrap();
    assert!(site.contains("working-directory: docs/site"), "{site}");
    assert!(site.contains("package_json_file: docs/site/package.json"), "{site}");
}

/// Rust is indented as rustfmt writes it, and the site as Starlight does.
#[test]
fn the_editor_settings_agree_with_what_the_tools_write() {
    let dir = tempfile::tempdir().unwrap();
    old_cli(dir.path());
    adopt(dir.path()).assert().success();
    let settings = fs::read_to_string(dir.path().join(".editorconfig")).unwrap();
    assert!(settings.contains("[*.rs]\nindent_size = 4"), "{settings}");
    assert!(settings.contains("[docs/**.{astro,mjs,ts}]\nindent_style = tab"), "{settings}");
}

/// The doctor reads what git tracks, so it stays red until the commit.
#[test]
fn it_says_the_doctor_sees_the_files_once_they_are_committed() {
    let dir = tempfile::tempdir().unwrap();
    old_cli(dir.path());
    let out = adopt(dir.path()).assert().success().get_output().stdout.clone();
    assert!(String::from_utf8_lossy(&out).contains("once they are committed"));
}
