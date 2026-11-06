//! `lyrn adr`, `lyrn docs`, `lyrn readme snap` and `lyrn changelog`, run as a user
//! runs them, on repositories made for the purpose.
//!
//! The program whose `--help` the pages are written from is lyrn itself, copied
//! into the fixture's `target/debug` under the fixture's name: a real clap program
//! with real commands, and no build to wait for.

use std::fs;
use std::path::Path;
use std::time::SystemTime;

mod common;

const NAME: &str = "demo";

fn write(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn read(root: &Path, path: &str) -> String {
    fs::read_to_string(root.join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// A crate on clap, with a built program and a Starlight site with a sidebar.
fn repository() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(
        root,
        "Cargo.toml",
        &format!("[package]\nname = \"{NAME}\"\nversion = \"1.2.0\"\nedition = \"2024\"\n\n[dependencies]\nclap = \"4\"\n"),
    );
    write(root, "src/main.rs", "fn main() {}\n");
    write(
        root,
        "docs/astro.config.mjs",
        "export default { integrations: [starlight({ sidebar: [{ items: [{ autogenerate: { directory: 'reference' } }] }] })] };\n",
    );
    write(root, "docs/src/content/docs/index.md", "---\ntitle: Home\ndescription: The start.\n---\n");
    let program = root.join("target/debug").join(format!("{NAME}{}", std::env::consts::EXE_SUFFIX));
    fs::create_dir_all(program.parent().unwrap()).unwrap();
    fs::copy(env!("CARGO_BIN_EXE_lyrn"), &program).unwrap();
    // A copy keeps the original's time on some systems: the program has to be
    // newer than the sources, or lyrn builds it again.
    fs::File::options().write(true).open(&program).unwrap().set_modified(SystemTime::now()).unwrap();
    dir
}

/// `lyrn <args>` in a directory; what it printed, and whether it succeeded. The
/// stand-in program was just written, and on Linux running it while another test's
/// fork still holds it open fails with "Text file busy": that flake, and only it,
/// is tried again.
fn lyrn(root: &Path, args: &[&str]) -> (String, bool) {
    for _ in 0..5 {
        let out = common::lyrn().current_dir(root).env_remove("CARGO_TARGET_DIR").args(args).output().unwrap();
        let printed = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
        if printed.contains("Text file busy") {
            std::thread::sleep(std::time::Duration::from_millis(200));
            continue;
        }
        return (printed, out.status.success());
    }
    panic!("the stand-in program stayed busy");
}

// --- adr --------------------------------------------------------------------

#[test]
fn a_decision_takes_the_next_number_and_a_row_in_the_index() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(root, "docs/adr/0001-first.md", "# ADR 0001: First\n");
    write(root, "docs/adr/0003-third.md", "# ADR 0003: Third\n");
    write(
        root,
        "docs/adr/README.md",
        "# Decisions\n\n| # | Decision | Status |\n| --- | --- | --- |\n| [0001](0001-first.md) | First | Accepted |\n| [0003](0003-third.md) | Third | Accepted |\n",
    );

    let (out, ok) = lyrn(root, &["adr", "new", "One service form, not two"]);
    assert!(ok, "{out}");
    let record = read(root, "docs/adr/0004-one-service-form-not-two.md");
    assert!(record.starts_with("# ADR 0004: One service form, not two\n"), "{record}");
    assert!(record.contains("- Status: accepted"), "{record}");
    for part in ["## Context", "## Decision", "## Consequences"] {
        assert!(record.contains(part), "no {part}:\n{record}");
    }
    let index = read(root, "docs/adr/README.md");
    assert!(
        index.ends_with("| [0003](0003-third.md) | Third | Accepted |\n| [0004](0004-one-service-form-not-two.md) | One service form, not two | Accepted |\n"),
        "{index}"
    );

    // The next one sees the file just written, not the number it started with.
    let (out, ok) = lyrn(root, &["adr", "new", "And another"]);
    assert!(ok, "{out}");
    assert!(root.join("docs/adr/0005-and-another.md").is_file());
}

#[test]
fn a_decision_needs_a_log_and_a_name_for_its_file() {
    let dir = tempfile::tempdir().unwrap();
    let (out, ok) = lyrn(dir.path(), &["adr", "new", "Anything"]);
    assert!(!ok);
    assert!(out.contains("`lyrn adopt` starts the log"), "{out}");

    write(dir.path(), "docs/adr/0001-first.md", "# First\n");
    let (out, ok) = lyrn(dir.path(), &["adr", "new", "Только русский"]);
    assert!(!ok);
    assert!(out.contains("write the title in English"), "{out}");

    // An index that is the repository's own is left as it is, and said so.
    write(dir.path(), "docs/adr/README.md", "Our decisions, in prose.\n");
    let (out, ok) = lyrn(dir.path(), &["adr", "new", "Second"]);
    assert!(ok, "{out}");
    assert!(out.contains("left as it was"), "{out}");
    assert_eq!(read(dir.path(), "docs/adr/README.md"), "Our decisions, in prose.\n");
}

// --- docs -------------------------------------------------------------------

#[test]
fn a_command_s_page_is_written_from_its_help() {
    let repo = repository();
    let root = repo.path();

    let (out, ok) = lyrn(root, &["docs", "ref", "template"]);
    assert!(ok, "{out}");
    let page = read(root, "docs/src/content/docs/reference/template.md");
    assert!(
        page.starts_with(&format!(
            "---\ntitle: {NAME} template\ndescription: \"Check a template, or write a built-in form out as one.\"\n---\n"
        )),
        "{page}"
    );
    assert!(page.contains(&format!("## {NAME} template export")), "{page}");
    assert!(page.contains(&format!("$ {NAME} template export [OPTIONS] <FORM>")), "{page}");
    assert!(!page.contains(".exe"), "{page}");
    assert!(page.contains("| `--repo <OWNER/NAME>` |"), "{page}");
    assert!(page.contains("The form to write out"), "{page}");

    // A page someone may have written since is not written over.
    let (out, ok) = lyrn(root, &["docs", "ref", "template"]);
    assert!(!ok);
    assert!(out.contains("already there"), "{out}");

    let (out, ok) = lyrn(root, &["docs", "ref", "nonsense"]);
    assert!(!ok);
    assert!(out.contains("no command `nonsense`"), "{out}");

    // With none named: every page missing, and the ones there are left alone.
    let before = read(root, "docs/src/content/docs/reference/template.md");
    let (out, ok) = lyrn(root, &["docs", "ref"]);
    assert!(ok, "{out}");
    assert!(root.join("docs/src/content/docs/reference/new.md").is_file(), "{out}");
    assert_eq!(read(root, "docs/src/content/docs/reference/template.md"), before);
    let (out, ok) = lyrn(root, &["docs", "ref"]);
    assert!(ok, "{out}");
    assert!(out.contains("Every command has a page"), "{out}");
}

/// Pages written from the program's help pass a check against the program: the
/// two have to agree, or the first draft would start out red.
#[test]
fn what_ref_writes_passes_check_and_a_flag_taken_off_fails_it() {
    let repo = repository();
    let root = repo.path();
    let (out, ok) = lyrn(root, &["docs", "ref"]);
    assert!(ok, "{out}");
    let (out, ok) = lyrn(root, &["docs", "check"]);
    assert!(ok, "{out}");
    assert!(out.contains("✓ flags"), "{out}");

    let file = "docs/src/content/docs/reference/deps.md";
    let page = read(root, "docs/src/content/docs/reference/check.md");
    write(root, "docs/src/content/docs/reference/check.md", &page.replace("--install", "--instal"));
    let (out, ok) = lyrn(root, &["docs", "check"]);
    assert!(!ok, "{out}");
    assert!(out.contains(&format!("`{NAME} check --install` is not on the command's page")), "{out}");
    write(root, "docs/src/content/docs/reference/check.md", &page);

    // A subcommand with no word on its parent's page.
    let page = read(root, "docs/src/content/docs/reference/template.md");
    write(
        root,
        "docs/src/content/docs/reference/template.md",
        &page.replace(&format!("{NAME} template export"), "the export"),
    );
    let (out, ok) = lyrn(root, &["docs", "check"]);
    assert!(!ok, "{out}");
    assert!(
        out.contains(&format!("`{NAME} template export` is not on the page of `{NAME} template`")),
        "{out}"
    );
    write(root, "docs/src/content/docs/reference/template.md", &page);

    // A page for a command that is gone.
    write(
        root,
        "docs/src/content/docs/reference/gone.md",
        &format!("---\ntitle: {NAME} gone\ndescription: Gone.\n---\n"),
    );
    let (out, ok) = lyrn(root, &["docs", "check"]);
    assert!(!ok, "{out}");
    assert!(out.contains("gone.md is the page of"), "{out}");
    fs::remove_file(root.join("docs/src/content/docs/reference/gone.md")).unwrap();

    // A link to nothing, and to a heading a page does not have.
    let page = read(root, file);
    write(
        root,
        file,
        &format!("{page}\nSee [nothing](/reference/nothing/), [a heading](/reference/check/#no-such) and [a file](/missing.svg).\n"),
    );
    let (out, ok) = lyrn(root, &["docs", "check"]);
    assert!(!ok, "{out}");
    assert!(out.contains("/reference/nothing/ is no page of the site"), "{out}");
    assert!(out.contains("/reference/check/#no-such names a heading the page does not have"), "{out}");
    assert!(out.contains("/missing.svg is no file of the site"), "{out}");
    write(root, file, &page);

    let (out, ok) = lyrn(root, &["docs", "check"]);
    assert!(ok, "{out}");
}

#[test]
fn a_guide_and_a_concept_are_started_and_the_sidebar_asked_about() {
    let repo = repository();
    let root = repo.path();
    let (out, ok) = lyrn(root, &["docs", "guide", "Your first release", "--description", "From a green CI to a tag."]);
    assert!(ok, "{out}");
    let page = read(root, "docs/src/content/docs/guides/your-first-release.md");
    assert!(
        page.starts_with("---\ntitle: \"Your first release\"\ndescription: \"From a green CI to a tag.\"\n---\n"),
        "{page}"
    );
    assert!(out.contains("lists no `guides/`"), "{out}");

    // Written over the sidebar it warns about, and the check says the same.
    let (out, ok) = lyrn(root, &["docs", "check"]);
    assert!(out.contains("no page of guides/ is in the sidebar"), "{out}");
    assert!(!out.contains("✗ sidebar"), "a warning, not a failure:\n{out}");
    let _ = ok;

    let (out, ok) = lyrn(root, &["docs", "concept", "What a stage is"]);
    assert!(ok, "{out}");
    assert!(out.contains("no description yet"), "{out}");
    let (out, _) = lyrn(root, &["docs", "check"]);
    assert!(out.contains("what-a-stage-is.md has no description"), "{out}");

    let (out, ok) = lyrn(root, &["docs", "guide", "Your first release"]);
    assert!(!ok);
    assert!(out.contains("already there"), "{out}");
}

#[test]
fn a_repository_with_no_site_is_pointed_at_the_form() {
    let dir = tempfile::tempdir().unwrap();
    for args in [&["docs", "check"][..], &["docs", "dev"][..], &["docs", "guide", "A"][..], &["docs", "ref"][..]] {
        let (out, ok) = lyrn(dir.path(), args);
        assert!(!ok, "{args:?}");
        assert!(out.contains("--form docs"), "{args:?}: {out}");
    }
}

/// `dev` runs the site's own commands where the site is, installing first only
/// when it has not been. A stand-in pnpm keeps what it was asked.
#[test]
fn the_site_is_served_from_where_it_lives_and_installed_first() {
    let repo = repository();
    let root = repo.path();
    let bin = root.join("stand-in");
    let log = root.join("pnpm.log");
    let logged = log.display().to_string();
    write(root, "stand-in/pnpm", &format!("#!/bin/sh\npwd >> \"{logged}\"\necho \"$1\" >> \"{logged}\"\n"));
    write(
        root,
        "stand-in/pnpm.cmd",
        &format!("@echo off\r\ncd >> \"{logged}\"\r\necho %1>> \"{logged}\"\r\n"),
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(bin.join("pnpm"), fs::Permissions::from_mode(0o755)).unwrap();
    }
    let path = std::env::join_paths(std::iter::once(bin.clone()).chain(std::env::split_paths(&std::env::var_os("PATH").unwrap()))).unwrap();

    let run = || common::lyrn().current_dir(root).env("PATH", &path).args(["docs", "dev"]).output().unwrap();
    assert!(run().status.success());
    let said = fs::read_to_string(&log).unwrap();
    let words: Vec<&str> = said.lines().filter(|l| *l == "install" || *l == "dev").collect();
    assert_eq!(words, ["install", "dev"], "{said}");
    assert!(
        said.lines().next().unwrap().replace('\\', "/").ends_with("/docs"),
        "not run in the site:\n{said}"
    );

    fs::remove_file(&log).unwrap();
    fs::create_dir_all(root.join("docs/node_modules")).unwrap();
    assert!(run().status.success());
    let said = fs::read_to_string(&log).unwrap();
    let words: Vec<&str> = said.lines().filter(|l| *l == "install" || *l == "dev").collect();
    assert_eq!(words, ["dev"], "installed again:\n{said}");
}

// --- readme snap ------------------------------------------------------------

#[test]
fn a_marked_transcript_is_taken_again_and_the_rest_left_alone() {
    let repo = repository();
    let root = repo.path();
    let readme = format!(
        "# Demo\n\n```console\n$ {NAME} <usage> stays\n```\n\n<!-- snap -->\n```console\n$ mkdir work && cd work\n$ pwd\nstale\n```\n\nWords between.\n\n<!-- snap: keep ^kept -->\n```console\n$ pwd && printf 'kept one\\ndropped\\nkept two\\n'\n```\n\n<!-- snap -->\n```console\n$ {NAME} --version\nstale\n```\n"
    );
    write(root, "README.md", &readme);

    // Stale, and --check says so and writes nothing.
    let (out, ok) = lyrn(root, &["readme", "snap", "--check"]);
    assert!(!ok, "{out}");
    assert!(out.contains("no longer what the build prints"), "{out}");
    assert_eq!(read(root, "README.md"), readme);

    let (out, ok) = lyrn(root, &["readme", "snap"]);
    assert!(ok, "{out}");
    let taken = read(root, "README.md");
    assert!(
        taken.contains(&format!("```console\n$ {NAME} <usage> stays\n```")),
        "an unmarked block was touched:\n{taken}"
    );
    // A `cd` holds for the next command, and the scratch directory is `~`.
    assert!(taken.contains("$ mkdir work && cd work\n$ pwd\n~/work\n```"), "{taken}");
    assert!(taken.contains("kept one\nkept two\n```") && !taken.contains("dropped\n"), "{taken}");
    assert!(taken.contains("Words between."), "{taken}");
    // The product's own build runs, first on the PATH.
    assert!(
        taken.contains(&format!("$ {NAME} --version\nlyrn {}\n```", env!("CARGO_PKG_VERSION"))),
        "{taken}"
    );

    // Taken again, nothing changes, and --check agrees.
    let (out, ok) = lyrn(root, &["readme", "snap", "--check"]);
    assert!(ok, "{out}");
    assert!(out.contains("as the build prints them"), "{out}");
    assert_eq!(read(root, "README.md"), taken);
}

#[test]
fn a_transcript_that_fails_leaves_the_readme_as_it_was() {
    let repo = repository();
    let root = repo.path();
    let readme = "<!-- snap -->\n```console\n$ echo printed; exit 3\nold\n```\n";
    write(root, "README.md", readme);
    let (out, ok) = lyrn(root, &["readme", "snap"]);
    assert!(!ok, "{out}");
    assert!(out.contains("failed, and README.md was left as it was") && out.contains("printed"), "{out}");
    assert_eq!(read(root, "README.md"), readme);

    write(root, "README.md", "No marks here.\n");
    let (out, ok) = lyrn(root, &["readme", "snap"]);
    assert!(ok, "{out}");
    assert!(out.contains("marks no transcript"), "{out}");
}

// --- changelog --------------------------------------------------------------

const CHANGES: &str = "# Changelog\n\nWords.\n\n## 🎉 [1.2.0] - 2026-11-02\n\n### ✨ Features\n\n- Two\n\n## 🎉 [1.1.0] - 2026-10-02\n\n- One\n";

#[test]
fn a_version_s_section_is_printed_and_given_its_screens() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(
        root,
        "Cargo.toml",
        "[package]\nname = \"demo\"\nversion = \"1.2.0\"\nrepository = \"https://github.com/acme/demo\"\n",
    );
    write(root, "CHANGELOG.md", CHANGES);

    let (out, ok) = lyrn(root, &["changelog"]);
    assert!(ok, "{out}");
    assert!(out.starts_with("## 🎉 [1.2.0]") && !out.contains("[1.1.0]"), "{out}");
    let (out, ok) = lyrn(root, &["changelog", "v1.1.0"]);
    assert!(ok, "{out}");
    assert!(out.contains("- One") && !out.contains("- Two"), "{out}");
    let (out, ok) = lyrn(root, &["changelog", "9.9.9"]);
    assert!(!ok);
    assert!(out.contains("no section for 9.9.9"), "{out}");

    let (out, ok) = lyrn(root, &["changelog", "--screens"]);
    assert!(!ok);
    assert!(out.contains("docs/screens/v1.2.0/"), "{out}");
    assert_eq!(read(root, "CHANGELOG.md"), CHANGES);

    write(root, "docs/screens/v1.2.0/02-its-list-view.png", "");
    write(root, "docs/screens/v1.2.0/01-the-new-board.png", "");
    write(root, "docs/screens/v1.2.0/notes.txt", "");
    let (out, ok) = lyrn(root, &["changelog", "--screens"]);
    assert!(ok, "{out}");
    assert!(out.contains("2 screens under v1.2.0"), "{out}");
    let written = read(root, "CHANGELOG.md");
    let base = "https://raw.githubusercontent.com/acme/demo/v1.2.0/docs/screens/v1.2.0";
    assert!(
        written.contains(&format!(
            "- Two\n\n### 🖼️ Screens\n\n![The new board]({base}/01-the-new-board.png)\n\n![Its list view]({base}/02-its-list-view.png)\n\n## 🎉 [1.1.0]"
        )),
        "{written}"
    );
    assert!(!written.contains("notes.txt"), "{written}");

    // Again, with nothing new: nothing changes.
    let (out, ok) = lyrn(root, &["changelog", "--screens"]);
    assert!(ok, "{out}");
    assert!(out.contains("already shows"), "{out}");
    assert_eq!(read(root, "CHANGELOG.md"), written);
}

#[test]
fn screens_need_somewhere_to_be_linked_from() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(root, "CHANGELOG.md", CHANGES);
    write(root, "docs/screens/v1.2.0/board.png", "");
    let (out, ok) = lyrn(root, &["changelog", "--screens"]);
    assert!(!ok);
    assert!(out.contains("absolute address"), "{out}");
    assert_eq!(read(root, "CHANGELOG.md"), CHANGES);
}
