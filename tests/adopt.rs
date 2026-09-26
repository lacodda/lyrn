//! `lyrn adopt`: an existing repository brought up to the line's standard.

use std::fs;
use std::path::Path;

use assert_cmd::Command;

fn adopt(dir: &Path) -> Command {
    let mut cmd = Command::cargo_bin("lyrn").unwrap();
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

/// A first ADR the repository already has keeps its number: a second 0001
/// would make "ADR 1" mean two things.
#[test]
fn an_adr_of_its_own_keeps_the_first_number() {
    let dir = tempfile::tempdir().unwrap();
    old_cli(dir.path());
    write(dir.path(), "docs/adr/0001-use-rust.md", "# Use Rust\n");
    adopt(dir.path()).assert().success();
    assert!(!dir.path().join("docs/adr/0001-record-architecture-decisions.md").exists());
    assert!(dir.path().join("docs/adr/README.md").is_file());
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
