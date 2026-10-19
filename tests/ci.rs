//! `lyrn ci`, run on repositories made for the purpose: a workflow whose
//! steps say what they saw, so the test can tell a clean checkout from the
//! work tree, an output passed on from one dropped, and a step that ran from
//! one that was never meant to.
//!
//! The steps are bash, which every runner and every development machine of
//! the line has - Git's on Windows.

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

fn write(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn git(root: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args([
            "-c",
            "user.name=Ada Lovelace",
            "-c",
            "user.email=ada@example.com",
            "-c",
            "init.defaultBranch=main",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .current_dir(root)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
}

/// A repository with one commit, a file the commit tracks and one it does
/// not, and the workflows given.
fn repository(workflows: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    git(root, &["init", "--quiet"]);
    write(root, "tracked.txt", "committed\n");
    for (name, text) in workflows {
        write(root, &format!(".github/workflows/{name}"), text);
    }
    git(root, &["add", "-A"]);
    git(root, &["commit", "--quiet", "--no-verify", "-m", "feat: start"]);
    write(root, "untracked.txt", "only in the work tree\n");
    dir
}

/// `lyrn ci` in `root`, its copies kept under `cache`.
fn ci(root: &Path, cache: &Path, args: &[&str]) -> (String, bool) {
    let out = common::lyrn()
        .arg("ci")
        .args(args)
        .arg("--path")
        .arg(root)
        .env("LOCALAPPDATA", cache)
        .env("XDG_CACHE_HOME", cache)
        .env("HOME", cache)
        .env_remove("CI")
        .output()
        .unwrap();
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    (text, out.status.success())
}

const CI: &str = r#"name: CI
on:
  push:
    branches: [main]
env:
  GREETING: from the workflow
jobs:
  gate:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
    defaults:
      run:
        shell: bash
    steps:
      - uses: actions/checkout@v7
      - id: version
        run: echo "number=1.2.3" >> "$GITHUB_OUTPUT"
      - name: Outputs pass on
        run: test "${{ steps.version.outputs.number }}" = "1.2.3"
      - name: The commit is checked out, and only the commit
        run: test -f tracked.txt && test ! -f untracked.txt
      - name: The runner's environment is set
        run: test "$CI" = "true" && test "$GREETING" = "from the workflow" && test -d "$RUNNER_TEMP"
      - name: Never published from here
        run: cargo publish --no-verify
      - name: Needs a secret
        run: echo "${{ secrets.TOKEN }}"
  after:
    needs: gate
    runs-on: ubuntu-latest
    steps:
      - name: Fails
        shell: bash
        run: exit 3
  skipped:
    needs: after
    runs-on: ubuntu-latest
    steps:
      - shell: bash
        run: echo never
"#;

#[test]
fn a_push_runs_here_on_the_commit_and_says_how_it_went() {
    let repo = repository(&[("ci.yml", CI)]);
    let cache = tempfile::tempdir().unwrap();
    let (report, passed) = ci(repo.path(), cache.path(), &[]);
    assert!(!passed, "a failing job passed:\n{report}");

    // The row for this system ran, every step of it that may.
    for line in [
        "✓ Outputs pass on",
        "✓ The commit is checked out, and only the commit",
        "✓ The runner's environment is set",
        "- Never published from here - publishes past this machine - lyrn ci never runs that",
        "- Needs a secret - its script needs `secrets`, which only GitHub has",
        "+ 2 more rows on",
    ] {
        assert!(report.contains(line), "no `{line}` in:\n{report}");
    }
    assert!(
        report.lines().any(|l| l.trim_start().starts_with("✓ gate (") && l.contains("passed")),
        "{report}"
    );
    // A job that fails fails the run; one that needs it does not run.
    assert!(report.contains("✗ after failed at \"Fails\""), "{report}");
    assert!(report.contains("- skipped - needs after, which did not pass"), "{report}");
    assert!(report.contains("1 job failed, 1 job passed"), "{report}");
    // The work tree is as it was.
    assert!(repo.path().join("untracked.txt").is_file());
}

#[test]
fn only_the_jobs_named_run() {
    let repo = repository(&[("ci.yml", CI)]);
    let cache = tempfile::tempdir().unwrap();
    let (report, passed) = ci(repo.path(), cache.path(), &["after"]);
    assert!(!passed, "{report}");
    assert!(!report.contains("Outputs pass on"), "{report}");
    let (refused, _) = ci(repo.path(), cache.path(), &["nonexistent"]);
    assert!(refused.contains("no workflow of this commit has a job `nonexistent`"), "{refused}");
}

/// A program this machine lacks is not the commit's fault, and not a pass:
/// the job could not run here, and the steps after it do not pretend to.
#[test]
fn a_missing_program_blocks_the_job_rather_than_failing_it() {
    let workflow = "on: push\njobs:\n  msrv:\n    runs-on: ubuntu-latest\n    defaults:\n      run:\n        shell: bash\n    steps:\n      - id: msrv\n        run: echo \"version=$(no-such-program-lyrn-test --version)\" >> \"$GITHUB_OUTPUT\"\n      - uses: dtolnay/rust-toolchain@master\n        with:\n          toolchain: ${{ steps.msrv.outputs.version }}\n      - name: Build\n        run: echo built\n";
    let repo = repository(&[("ci.yml", workflow)]);
    let cache = tempfile::tempdir().unwrap();
    let (report, passed) = ci(repo.path(), cache.path(), &[]);
    assert!(!passed, "a job that could not run passed:\n{report}");
    assert!(report.contains("needs `no-such-program-lyrn-test`, which is not installed here"), "{report}");
    assert!(report.contains("- Build - not run: a step before it could not run here"), "{report}");
    assert!(report.contains("1 job could not run here"), "{report}");
    assert!(!report.contains("✓ Build"), "{report}");
}

#[test]
fn list_says_what_a_push_would_start_and_runs_nothing() {
    let release = "on:\n  push:\n    tags: ['v*']\njobs:\n  build:\n    runs-on: ubuntu-latest\n    steps:\n      - run: exit 1\n";
    let docs = "on:\n  push:\n    paths: ['docs/**']\njobs:\n  site:\n    runs-on: ubuntu-latest\n    steps:\n      - run: exit 1\n";
    let manual = "on: workflow_dispatch\njobs:\n  x:\n    runs-on: ubuntu-latest\n    steps:\n      - run: exit 1\n";
    let repo = repository(&[("ci.yml", CI), ("release.yml", release), ("docs.yml", docs), ("manual.yml", manual)]);
    // Pushed already: the push is the last commit, which changed no docs.
    let remote = tempfile::tempdir().unwrap();
    git(remote.path(), &["init", "--quiet", "--bare"]);
    git(repo.path(), &["remote", "add", "origin", &remote.path().display().to_string()]);
    git(repo.path(), &["push", "--quiet", "-u", "origin", "main"]);
    let cache = tempfile::tempdir().unwrap();
    let (report, passed) = ci(repo.path(), cache.path(), &["--list"]);
    assert!(passed, "{report}");
    assert!(
        report.contains("release.yml - not started: it starts on a pushed tag, not a branch"),
        "{report}"
    );
    assert!(report.contains("docs.yml - not started: it starts when docs/** change"), "{report}");
    assert!(
        report.contains("manual.yml - not started: it starts on workflow_dispatch, not on a push"),
        "{report}"
    );
    assert!(report.contains("runs here"), "{report}");
    assert!(!cache.path().join("lyrn").exists(), "--list made a copy");
}
