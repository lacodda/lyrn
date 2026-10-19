//! `lyrn stage`, `status`, `versions` and `deps`, run on a repository made
//! for the purpose, with a record and registries of the test's own: a
//! stand-in rigger that writes down what it was told, and a local server in
//! place of crates.io, npm and GitHub.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;

mod common;

fn write(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn git(root: &Path, args: &[&str], date: Option<&str>) {
    let mut command = Command::new("git");
    command
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
        .env("GIT_CONFIG_NOSYSTEM", "1");
    if let Some(date) = date {
        command.env("GIT_AUTHOR_DATE", date).env("GIT_COMMITTER_DATE", date);
    }
    let out = command.output().unwrap();
    assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
}

/// A released product: a crate with an npm wrapper at 1.2.0, tagged, its
/// stack last brought up to date on `updated`.
fn repository(updated: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    git(root, &["init", "--quiet"], None);
    write(
        root,
        "Cargo.toml",
        "[package]\nname = \"demo\"\nversion = \"1.2.0\"\nedition = \"2024\"\ndescription = \"Count the words\"\nrepository = \"https://github.com/someone/demo\"\n\n[dependencies]\nserde = \"1\"\n",
    );
    write(root, "src/lib.rs", "");
    write(
        root,
        "Cargo.lock",
        "version = 4\n\n[[package]]\nname = \"demo\"\nversion = \"1.2.0\"\ndependencies = [\n \"serde\",\n]\n\n[[package]]\nname = \"serde\"\nversion = \"1.0.200\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n",
    );
    write(
        root,
        "npm/package.json",
        "{\n  \"name\": \"demo-cli\",\n  \"version\": \"1.2.0\",\n  \"demo\": { \"binary\": \"v1.2.0\" },\n  \"description\": \"Count the words\"\n}\n",
    );
    write(root, "CHANGELOG.md", "# Changelog\n\n## [1.2.0] - 2026-10-01\n\n- Counting\n");
    write(root, "README.md", "> Count the words.\n");
    git(root, &["add", "-A"], None);
    git(
        root,
        &["commit", "--quiet", "--no-verify", "-m", "chore(deps): update the toolchain and dependencies"],
        Some(updated),
    );
    git(root, &["tag", "v1.2.0"], None);
    dir
}

/// A rigger that knows the repository as `demo`, is building `v1.3.0`, and
/// writes every note it is given into `notes.txt` beside itself.
struct Record {
    dir: tempfile::TempDir,
}

impl Record {
    fn new(repo: &Path) -> Record {
        let dir = tempfile::tempdir().unwrap();
        let path = repo.display().to_string().replace('\\', "/");
        write(
            dir.path(),
            "projects.json",
            &format!("[{{\"name\":\"demo\",\"path\":\"{path}\",\"kind\":\"repo\",\"tier\":null}}]\n"),
        );
        write(
            dir.path(),
            "context.json",
            "{\"project\":\"demo\",\"current\":{\"version\":\"v1.3.0\",\"title\":\"Counting lines\",\"tasks\":[{\"id\":7,\"title\":\"**Lines** too\",\"status\":\"new\"}]}}\n",
        );
        if cfg!(windows) {
            write(
                dir.path(),
                "rigger.cmd",
                "@echo off\r\nif \"%2\"==\"project\" type \"%~dp0projects.json\" & exit /b 0\r\nif \"%2\"==\"context\" type \"%~dp0context.json\" & exit /b 0\r\nif \"%1\"==\"note\" echo %* >> \"%~dp0notes.txt\" & exit /b 0\r\nexit /b 1\r\n",
            );
        } else {
            write(
                dir.path(),
                "rigger",
                "#!/bin/sh\nhere=$(dirname \"$0\")\ncase \"$2\" in\n  project) cat \"$here/projects.json\"; exit 0 ;;\n  context) cat \"$here/context.json\"; exit 0 ;;\nesac\nif [ \"$1\" = note ]; then echo \"$*\" >> \"$here/notes.txt\"; exit 0; fi\nexit 1\n",
            );
            executable(&dir.path().join("rigger"));
        }
        Record { dir }
    }

    fn program(&self) -> PathBuf {
        self.dir.path().join(if cfg!(windows) { "rigger.cmd" } else { "rigger" })
    }

    fn notes(&self) -> String {
        fs::read_to_string(self.dir.path().join("notes.txt")).unwrap_or_default()
    }
}

#[cfg(unix)]
fn executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

#[cfg(not(unix))]
fn executable(_: &Path) {}

/// A server answering every question with what `answer` makes of its path,
/// for as long as the test runs.
fn server(answer: fn(&str) -> Option<String>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut request = String::new();
            let _ = BufReader::new(&stream).read_line(&mut request);
            let path = request.split_whitespace().nth(1).unwrap_or("/").to_string();
            let response = match answer(&path) {
                Some(body) => format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()),
                None => "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_string(),
            };
            let _ = stream.write_all(response.as_bytes());
        }
    });
    address
}

/// Nothing listens here: every question is refused at once.
const NOWHERE: &str = "http://127.0.0.1:9";

/// lyrn with the record and the network the test gives it.
fn lyrn(record: Option<&Record>, registries: &str) -> assert_cmd::Command {
    let mut cmd = common::lyrn();
    cmd.env("LYRN_RIGGER", record.map_or_else(|| PathBuf::from("no-such-rigger-lyrn-test"), Record::program))
        .env("LYRN_GITHUB_API", NOWHERE)
        .env("LYRN_GITHUB_GIT", registries)
        .env("LYRN_CRATES_INDEX", registries)
        .env("LYRN_NPM_REGISTRY", registries)
        .env("GH_TOKEN", "")
        .env("GITHUB_TOKEN", "");
    cmd
}

fn run(mut cmd: assert_cmd::Command) -> String {
    // The stand-in rigger is an executable the test has just written, and
    // on Linux running one while another test's fork still holds it open
    // fails with "Text file busy". That flake, and only it, is tried again.
    for _ in 0..5 {
        let out = cmd.output().unwrap();
        let stdout = String::from_utf8(out.stdout).unwrap();
        if stdout.contains("Text file busy") {
            std::thread::sleep(std::time::Duration::from_millis(200));
            continue;
        }
        assert!(out.status.success(), "{stdout}{}", String::from_utf8_lossy(&out.stderr));
        return stdout;
    }
    panic!("the stand-in rigger stayed busy");
}

#[test]
fn a_stage_starts_with_what_stands_in_its_way_and_a_line_in_the_record() {
    let repo = repository("2020-01-01T10:00:00+00:00");
    let record = Record::new(repo.path());
    let mut cmd = lyrn(Some(&record), NOWHERE);
    cmd.arg("stage").arg(repo.path());
    let report = run(cmd);
    for line in [
        "stage     v1.3.0 · Counting lines, from rigger's record",
        "[7] Lines too",
        "release   v1.2.0 on",
        "stack     updated",
        "the stage starts with one `chore(deps)` commit",
        "Before the stage's first commit:",
        "the stack is due: update it first, in one `chore(deps)` commit",
        "In rigger's record for demo: Stage v1.3.0 began at",
    ] {
        assert!(report.contains(line), "no `{line}` in:\n{report}");
    }
    let notes = record.notes();
    assert!(notes.contains("note demo --kind state"), "{notes}");
    assert!(notes.contains("Stage v1.3.0 began at"), "{notes}");
    assert!(notes.contains("stack due"), "{notes}");
}

#[test]
fn without_a_record_a_stage_is_reported_and_not_recorded() {
    let repo = repository("2099-01-01T10:00:00+00:00");
    let mut cmd = lyrn(None, NOWHERE);
    cmd.arg("stage").arg(repo.path());
    let report = run(cmd);
    assert!(report.contains("stage     not read - rigger is not installed here"), "{report}");
    assert!(report.contains("Not recorded - rigger is not installed here"), "{report}");
}

#[test]
fn status_says_where_the_repository_stands() {
    let repo = repository("2026-10-01T10:00:00+00:00");
    write(repo.path(), "notes.md", "draft\n");
    git(repo.path(), &["add", "notes.md"], None);
    git(repo.path(), &["commit", "--quiet", "--no-verify", "-m", "feat: count lines"], None);
    write(repo.path(), "scratch.txt", "x\n");
    let record = Record::new(repo.path());
    let mut cmd = lyrn(Some(&record), NOWHERE);
    cmd.arg("status").arg(repo.path());
    let report = run(cmd);
    for line in [
        "branch    main, tracking nothing - never pushed",
        "tree      1 file untracked",
        "released  v1.2.0 on",
        "1 commit since: 1 feat",
        "ci        nothing pushed to ask about",
        "stage     v1.3.0 · Counting lines, 1 task open",
    ] {
        assert!(report.contains(line), "no `{line}` in:\n{report}");
    }
    assert!(record.notes().is_empty(), "status wrote to the record");
}

fn registry(path: &str) -> Option<String> {
    match path {
        // The sparse index: one line per version, a yanked one and a
        // pre-release among them.
        "/se/rd/serde" => Some(
            "{\"name\":\"serde\",\"vers\":\"1.0.200\",\"yanked\":false}\n{\"name\":\"serde\",\"vers\":\"1.0.228\",\"yanked\":false}\n{\"name\":\"serde\",\"vers\":\"1.0.229\",\"yanked\":true}\n{\"name\":\"serde\",\"vers\":\"2.0.0-alpha\",\"yanked\":false}\n"
                .to_string(),
        ),
        "/de/mo/demo" => Some("{\"name\":\"demo\",\"vers\":\"1.2.0\",\"yanked\":false}\n".to_string()),
        "/demo-cli" => Some("{\"dist-tags\":{\"latest\":\"1.1.0\"}}".to_string()),
        _ => None,
    }
}

#[test]
fn versions_says_where_the_number_is_written_and_where_it_went() {
    let repo = repository("2026-10-01T10:00:00+00:00");
    let mut cmd = lyrn(None, &server(registry));
    cmd.arg("versions").arg(repo.path());
    let report = run(cmd);
    for line in [
        "  1.2.0   Cargo.toml",
        "  1.2.0   npm/package.json",
        "  v1.2.0  npm/package.json (demo.binary)",
        "  1.2.0   Cargo.lock (demo)",
        "  1.2.0   CHANGELOG.md, the newest section, 2026-10-01",
        "  v1.2.0  tagged",
        "nothing committed since",
        "  1.2.0   crates.io: demo",
        "  1.1.0   npm: demo-cli",
        "Every file says 1.2.0.",
    ] {
        assert!(report.contains(line), "no `{line}` in:\n{report}");
    }
}

#[test]
fn deps_puts_what_a_build_uses_beside_the_newest() {
    let repo = repository("2026-10-01T10:00:00+00:00");
    let mut cmd = lyrn(None, &server(registry));
    cmd.arg("deps").arg(repo.path());
    let report = run(cmd);
    assert!(report.contains("Cargo.toml - crates.io"), "{report}");
    // Neither the yanked version nor the pre-release is the newest.
    assert!(report.contains("serde  1.0.200  → 1.0.228"), "{report}");
    assert!(!report.contains("a breaking version"), "{report}");
    assert!(report.contains("1 dependency behind."), "{report}");
}
