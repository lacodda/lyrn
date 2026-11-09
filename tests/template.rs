//! Templates from outside the binary: a directory, your own under
//! ~/.lyrn/templates, a cargo-generate template, and a GitHub tag - the last
//! against a local repository and a local stand-in for GitHub's API, so the
//! CI check is exercised without the network.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command as Process;
use std::sync::{Arc, Mutex};

use predicates::prelude::*;

mod common;

use common::lyrn;

/// A built-in form written out as a template, in a directory of its own.
fn exported(form: &str, dir: &Path) -> PathBuf {
    let root = dir.join(format!("template-{form}"));
    lyrn()
        .args(["template", "export", form, "--repo", "owner/template-x", "--path"])
        .arg(&root)
        .assert()
        .success();
    root
}

fn read(path: impl AsRef<Path>) -> String {
    fs::read_to_string(path.as_ref()).unwrap_or_else(|e| panic!("{}: {e}", path.as_ref().display()))
}

fn edit(path: impl AsRef<Path>, from: &str, to: &str) {
    let text = read(&path);
    assert!(text.contains(from), "`{from}` is not in {}", path.as_ref().display());
    fs::write(path, text.replacen(from, to, 1)).unwrap();
}

#[test]
fn a_template_directory_is_used_by_path() {
    let dir = tempfile::tempdir().unwrap();
    let template = exported("spa", dir.path());
    let out = dir.path().join("demo-app");

    lyrn()
        .args([
            "new",
            "demo-app",
            "--yes",
            "--no-hooks",
            "--repo",
            "owner/demo-app",
            "--with",
            "router,pwa",
            "--template",
        ])
        .arg(&template)
        .arg("--path")
        .arg(&out)
        .assert()
        .success()
        .stdout(predicate::str::contains("Using the template in"));

    // An add-on's own files, and its lines in a shared one.
    assert!(out.join("public/sw.js").is_file());
    assert!(read(out.join("package.json")).contains("\"react-router\""));
    let manifest = read(out.join("lyrn.toml"));
    assert!(manifest.contains("source = \"path:template-spa\""), "{manifest}");
    assert!(manifest.contains("revision = \"unversioned\""), "{manifest}");
    assert!(manifest.contains("form = \"spa\""), "{manifest}");
}

/// A written-out form generates exactly what the built-in one does - the
/// file tree and every byte of it, save the line that says where it came
/// from.
#[test]
fn an_exported_form_generates_the_builtin_project() {
    let dir = tempfile::tempdir().unwrap();
    let template = exported("cli", dir.path());
    let common = ["--yes", "--no-hooks", "--repo", "owner/demo-tool", "--author", "Tester", "--with", "keyring"];
    let from_builtin = dir.path().join("builtin");
    let from_template = dir.path().join("template");
    lyrn()
        .args(["new", "demo-tool", "--form", "cli"])
        .args(common)
        .arg("--path")
        .arg(&from_builtin)
        .assert()
        .success();
    lyrn()
        .args(["new", "demo-tool", "--template"])
        .arg(&template)
        .args(common)
        .arg("--path")
        .arg(&from_template)
        .assert()
        .success();

    let files = |root: &Path| -> Vec<(String, Vec<u8>)> {
        let mut found = Vec::new();
        let mut stack = vec![root.to_path_buf()];
        while let Some(dir) = stack.pop() {
            for entry in fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    stack.push(path);
                } else {
                    let relative = path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
                    let text = fs::read(&path).unwrap();
                    // The one line that differs, by design.
                    let text = String::from_utf8_lossy(&text)
                        .lines()
                        .filter(|l| !l.starts_with("source = ") && !l.starts_with("revision = "))
                        .collect::<Vec<_>>()
                        .join("\n")
                        .into_bytes();
                    found.push((relative, text));
                }
            }
        }
        found.sort();
        found
    };
    assert_eq!(files(&from_template), files(&from_builtin));
}

#[test]
fn check_passes_a_good_template_and_names_what_is_wrong_with_a_bad_one() {
    let dir = tempfile::tempdir().unwrap();
    let template = exported("spa", dir.path());
    lyrn()
        .args(["template", "check"])
        .arg(&template)
        .assert()
        .success()
        .stdout(predicate::str::contains("is a lyrn template of the spa form"));

    fs::write(template.join("template/NOTES.md"), "{{ nonesuch }}\n").unwrap();
    lyrn()
        .args(["template", "check"])
        .arg(&template)
        .assert()
        .failure()
        .stderr(predicate::str::contains("`NOTES.md` asks for `{{ nonesuch }}`"));
}

/// A template written for a newer lyrn uses a field this one does not know.
/// Read past, it would generate something the template never meant.
#[test]
fn an_unknown_manifest_field_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let template = exported("spa", dir.path());
    edit(template.join("template.toml"), "form = \"spa\"", "form = \"spa\"\nlayouts = [\"wide\"]");
    lyrn()
        .args(["template", "check"])
        .arg(&template)
        .assert()
        .failure()
        .stderr(predicate::str::contains("layouts"));
}

#[test]
fn a_hook_outside_the_vocabulary_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let template = exported("spa", dir.path());
    let manifest = read(template.join("template.toml"));
    fs::write(
        template.join("template.toml"),
        manifest.replacen(
            "[[hooks]]",
            "[[hooks]]\nname = \"Phoning home\"\nrun = [\"curl\", \"https://example.com\"]\n\n[[hooks]]",
            1,
        ),
    )
    .unwrap();
    lyrn()
        .args(["new", "demo-app", "--yes", "--no-hooks", "--template"])
        .arg(&template)
        .arg("--path")
        .arg(dir.path().join("demo-app"))
        .assert()
        .failure()
        .stderr(predicate::str::contains("the hook `Phoning home` runs `curl https://example.com`"));
    assert!(!dir.path().join("demo-app").exists(), "a refused template wrote files");
}

#[test]
fn a_local_template_stands_in_for_the_form_it_is_named_after() {
    let home = tempfile::tempdir().unwrap();
    let templates = home.path().join("templates");
    fs::create_dir_all(&templates).unwrap();
    let written = exported("spa", home.path());
    fs::rename(&written, templates.join("spa")).unwrap();
    edit(templates.join("spa/template/README.md"), "", "<!-- mine -->\n");

    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("demo-app");
    // No `--form` at all: the default form is the spa, and yours wins.
    lyrn()
        .env("LYRN_HOME", home.path())
        .args(["new", "demo-app", "--yes", "--no-hooks", "--repo", "owner/demo-app", "--path"])
        .arg(&out)
        .assert()
        .success()
        .stdout(predicate::str::contains("in place of the built-in spa form"));
    assert!(read(out.join("README.md")).starts_with("<!-- mine -->"));
    assert!(read(out.join("lyrn.toml")).contains("source = \"local:spa\""));

    lyrn()
        .env("LYRN_HOME", home.path())
        .arg("forms")
        .assert()
        .success()
        .stdout(predicate::str::contains("the spa form, in place of the built-in one"));
}

#[test]
fn a_local_template_is_named_by_its_directory() {
    let home = tempfile::tempdir().unwrap();
    let templates = home.path().join("templates");
    fs::create_dir_all(&templates).unwrap();
    fs::rename(exported("cli", home.path()), templates.join("house-cli")).unwrap();

    let dir = tempfile::tempdir().unwrap();
    lyrn()
        .env("LYRN_HOME", home.path())
        .args(["new", "demo-tool", "--template", "house-cli", "--yes", "--no-hooks", "--path"])
        .arg(dir.path().join("demo-tool"))
        .assert()
        .success();
    assert!(dir.path().join("demo-tool/Cargo.toml").is_file());

    lyrn()
        .env("LYRN_HOME", home.path())
        .args(["new", "demo-tool", "--template", "nonesuch", "--yes", "--no-hooks"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("there is no template `nonesuch`"));
}

/// `~/.lyrn/templates/spa` that is not a spa would make `--form spa` mean
/// something else on this one machine.
#[test]
fn a_local_template_of_another_form_cannot_stand_in() {
    let home = tempfile::tempdir().unwrap();
    let templates = home.path().join("templates");
    fs::create_dir_all(&templates).unwrap();
    fs::rename(exported("cli", home.path()), templates.join("spa")).unwrap();
    lyrn()
        .env("LYRN_HOME", home.path())
        .args(["new", "demo-app", "--form", "spa", "--yes", "--no-hooks", "--dry-run"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("says `form = \"cli\"`"));
}

#[test]
fn form_and_template_are_one_or_the_other() {
    lyrn()
        .args(["new", "demo-app", "--form", "spa", "--template", "./x", "--dry-run"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn a_repository_without_a_tag_is_refused_before_anything_is_fetched() {
    lyrn()
        .args(["new", "demo-app", "--template", "lacodda/template-spa", "--dry-run"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("names no tag"));
}

fn write(root: &Path, path: &str, text: &str) {
    let target = root.join(path);
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    fs::write(target, text).unwrap();
}

/// A template written for cargo-generate: Liquid, its variables, its filters,
/// its placeholders answered with `--define`.
#[test]
fn a_cargo_generate_template_generates() {
    let dir = tempfile::tempdir().unwrap();
    let template = dir.path().join("cg");
    write(&template, "Cargo.toml", "[package]\nname = \"{{project-name}}\"\nedition = \"{{ edition }}\"\n");
    write(&template, "src/main.rs", "struct {{ project-name | pascal_case }};\nfn main() {}\n");
    write(
        &template,
        "cargo-generate.toml",
        "[placeholders.edition]\ntype = \"string\"\nprompt = \"Which edition?\"\nchoices = [\"2021\", \"2024\"]\ndefault = \"2021\"\n",
    );
    let out = dir.path().join("word-count");

    lyrn()
        .args(["new", "word-count", "--yes", "--no-hooks", "--define", "edition=2024", "--template"])
        .arg(&template)
        .arg("--path")
        .arg(&out)
        .assert()
        .success()
        .stdout(predicate::str::contains("cargo build"));
    assert_eq!(read(out.join("Cargo.toml")), "[package]\nname = \"word-count\"\nedition = \"2024\"\n");
    assert_eq!(read(out.join("src/main.rs")), "struct WordCount;\nfn main() {}\n");
    assert!(!out.join("cargo-generate.toml").exists(), "the template's own manifest was copied");
    // Not a project of the line: nothing claims otherwise.
    assert!(!out.join("lyrn.toml").exists());
}

#[test]
fn the_line_s_flags_mean_nothing_to_a_cargo_generate_template() {
    let dir = tempfile::tempdir().unwrap();
    let template = dir.path().join("cg");
    write(&template, "Cargo.toml", "[package]\nname = \"{{project-name}}\"\n");
    lyrn()
        .args(["new", "word-count", "--yes", "--no-hooks", "--accent", "kilna", "--template"])
        .arg(&template)
        .arg("--path")
        .arg(dir.path().join("out"))
        .assert()
        .failure()
        .stderr(predicate::str::contains("--accent means nothing to a cargo-generate template"));
}

// --- GitHub, stood in for by a local repository and a local API -----------

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Process::new("git")
        .args([
            "-c",
            "user.name=Tester",
            "-c",
            "user.email=tester@example.com",
            "-c",
            "init.defaultBranch=main",
            "-c",
            "core.autocrlf=false",
        ])
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// `remotes/owner/template-spa.git`, a bare repository whose tag `v1.0.0`
/// holds the exported spa form; and the commit the tag points at.
fn remote(remotes: &Path) -> String {
    let work = remotes.join("work");
    fs::create_dir_all(&work).unwrap();
    let template = exported("spa", &work);
    git(&template, &["init", "--quiet"]);
    git(&template, &["add", "."]);
    git(&template, &["commit", "--quiet", "-m", "feat: the spa form"]);
    git(&template, &["tag", "-a", "v1.0.0", "-m", "v1.0.0"]);
    let commit = git(&template, &["rev-parse", "HEAD"]);
    fs::create_dir_all(remotes.join("owner")).unwrap();
    git(remotes, &["clone", "--quiet", "--bare", template.to_str().unwrap(), "owner/template-spa.git"]);
    commit
}

/// A stand-in for GitHub's API that answers every question about check runs
/// with `check_runs`, and about commit statuses with none.
fn api(check_runs: &'static str) -> String {
    api_with(check_runs, || {}).0
}

/// The same, calling `on_ask` whenever the check runs are asked about, and
/// keeping the request lines it was sent.
fn api_with(check_runs: &'static str, on_ask: impl Fn() + Send + 'static) -> (String, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    let asked = Arc::new(Mutex::new(Vec::new()));
    let log = Arc::clone(&asked);
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut request_line = String::new();
            reader.read_line(&mut request_line).unwrap();
            // Read the rest of the head, so the client sees its request taken.
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap() == 0 || line == "\r\n" {
                    break;
                }
            }
            log.lock().unwrap().push(request_line.trim().to_string());
            let body = if request_line.contains("/check-runs") {
                on_ask();
                check_runs.to_string()
            } else {
                r#"{"state":"pending","total_count":0,"statuses":[]}"#.to_string()
            };
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
        }
    });
    (address, asked)
}

fn from_github(remotes: &Path, api: &str, spec: &str, out: &Path) -> assert_cmd::assert::Assert {
    lyrn()
        .env("LYRN_GITHUB_GIT", remotes)
        .env("LYRN_GITHUB_API", api)
        .args([
            "new",
            "demo-app",
            "--yes",
            "--no-hooks",
            "--repo",
            "owner/demo-app",
            "--template",
            spec,
            "--path",
        ])
        .arg(out)
        .assert()
}

const GREEN: &str = r#"{"total_count":2,"check_runs":[
    {"name":"check","status":"completed","conclusion":"success"},
    {"name":"generate","status":"completed","conclusion":"success"}]}"#;

#[test]
fn a_tag_whose_ci_passed_is_used_and_pinned() {
    let remotes = tempfile::tempdir().unwrap();
    let commit = remote(remotes.path());
    let out = remotes.path().join("demo-app");

    let (address, asked) = api_with(GREEN, || {});
    from_github(remotes.path(), &address, "owner/template-spa@v1.0.0", &out)
        .success()
        .stdout(predicate::str::contains("owner/template-spa@v1.0.0").and(predicate::str::contains("passed its CI")));

    // The CI is asked about the commit, never about the tag's name: a name
    // can be moved to other files, a commit cannot.
    let asked = asked.lock().unwrap();
    assert!(
        asked
            .iter()
            .any(|q| q.contains(&format!("/repos/owner/template-spa/commits/{commit}/check-runs"))),
        "{asked:?}"
    );
    assert!(!asked.iter().any(|q| q.contains("v1.0.0")), "{asked:?}");

    let manifest = read(out.join("lyrn.toml"));
    assert!(manifest.contains("source = \"owner/template-spa@v1.0.0\""), "{manifest}");
    // The annotated tag's commit, not the tag object: the CI ran on the one.
    assert!(manifest.contains(&format!("revision = \"{commit}\"")), "{manifest}");
}

#[test]
fn a_tag_whose_ci_failed_is_refused() {
    let remotes = tempfile::tempdir().unwrap();
    remote(remotes.path());
    let out = remotes.path().join("demo-app");
    let red = r#"{"total_count":2,"check_runs":[
        {"name":"check","status":"completed","conclusion":"success"},
        {"name":"generate (router)","status":"completed","conclusion":"failure"}]}"#;

    from_github(remotes.path(), &api(red), "owner/template-spa@v1.0.0", &out)
        .failure()
        .stderr(predicate::str::contains(
            "the CI of `owner/template-spa@v1.0.0` failed: generate (router) (failure)",
        ));
    assert!(!out.exists(), "a refused template wrote files");
}

#[test]
fn a_tag_without_ci_is_refused_with_the_way_round() {
    let remotes = tempfile::tempdir().unwrap();
    remote(remotes.path());
    from_github(
        remotes.path(),
        &api(r#"{"total_count":0,"check_runs":[]}"#),
        "owner/template-spa@v1.0.0",
        &remotes.path().join("demo-app"),
    )
    .failure()
    .stderr(predicate::str::contains("has no CI result").and(predicate::str::contains("--template ./template-spa")));
}

#[test]
fn a_tag_still_running_is_refused() {
    let remotes = tempfile::tempdir().unwrap();
    remote(remotes.path());
    let running = r#"{"total_count":1,"check_runs":[{"name":"generate","status":"in_progress","conclusion":null}]}"#;
    from_github(remotes.path(), &api(running), "owner/template-spa@v1.0.0", &remotes.path().join("demo-app"))
        .failure()
        .stderr(predicate::str::contains("still running (generate)"));
}

#[test]
fn a_tag_that_does_not_exist_is_refused_with_the_ones_that_do() {
    let remotes = tempfile::tempdir().unwrap();
    remote(remotes.path());
    from_github(remotes.path(), &api(GREEN), "owner/template-spa@v9", &remotes.path().join("demo-app"))
        .failure()
        .stderr(predicate::str::contains("has no tag `v9` (its newest: v1.0.0)"));
}

/// A dry run asks the same questions as the real one: it must not promise a
/// project the real run would refuse.
#[test]
fn a_dry_run_is_held_to_the_same_ci() {
    let remotes = tempfile::tempdir().unwrap();
    remote(remotes.path());
    lyrn()
        .env("LYRN_GITHUB_GIT", remotes.path())
        .env("LYRN_GITHUB_API", api(r#"{"total_count":0,"check_runs":[]}"#))
        .args(["new", "demo-app", "--dry-run", "--template", "owner/template-spa@v1.0.0"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("has no CI result"));
}

/// The tag is moved to other files after its CI was asked about and before
/// the clone: what would be generated is not what the CI built.
#[test]
fn a_tag_moved_after_its_ci_was_asked_is_refused() {
    let remotes = tempfile::tempdir().unwrap();
    remote(remotes.path());
    let template = remotes.path().join("work/template-spa");
    fs::write(template.join("template/NOTES.md"), "added after the CI ran\n").unwrap();
    git(&template, &["add", "."]);
    git(&template, &["commit", "--quiet", "-m", "chore: something else"]);
    let bare = remotes.path().join("owner/template-spa.git");
    git(&template, &["push", "--quiet", bare.to_str().unwrap(), "main"]);

    let moves = bare.clone();
    let (address, _) = api_with(GREEN, move || {
        git(&moves, &["tag", "-f", "-a", "v1.0.0", "-m", "moved", "main"]);
    });
    let out = remotes.path().join("demo-app");
    from_github(remotes.path(), &address, "owner/template-spa@v1.0.0", &out)
        .failure()
        .stderr(predicate::str::contains("moved while lyrn was reading it"));
    assert!(!out.exists(), "files from a moved tag were written");
}

// --- Upgrading a project made from a template, and following a form -------

/// A git config of its own for the processes a test runs, so a commit lyrn
/// makes has an author on any machine.
struct Identity {
    _dir: tempfile::TempDir,
    global: PathBuf,
}

impl Identity {
    fn new() -> Identity {
        let dir = tempfile::tempdir().unwrap();
        let global = dir.path().join(".gitconfig");
        fs::write(
            &global,
            "[user]\n\tname = Ada Lovelace\n\temail = ada@example.com\n[init]\n\tdefaultBranch = main\n[commit]\n\tgpgsign = false\n[core]\n\tautocrlf = false\n",
        )
        .unwrap();
        Identity { _dir: dir, global }
    }

    fn lyrn(&self, remotes: &Path, api: &str) -> assert_cmd::Command {
        let mut command = lyrn();
        command
            .env("GIT_CONFIG_GLOBAL", &self.global)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("LYRN_GITHUB_GIT", remotes)
            .env("LYRN_GITHUB_API", api);
        command
    }
}

/// A stand-in for GitHub's API whose answer about a commit's check runs is
/// `answer` of the request line: green for one commit, red for another.
fn api_answering(answer: impl Fn(&str) -> String + Send + 'static) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut request_line = String::new();
            reader.read_line(&mut request_line).unwrap();
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap() == 0 || line == "\r\n" {
                    break;
                }
            }
            let body = if request_line.contains("/check-runs") {
                answer(&request_line)
            } else {
                r#"{"state":"pending","total_count":0,"statuses":[]}"#.to_string()
            };
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
        }
    });
    address
}

const RED: &str = r#"{"total_count":2,"check_runs":[
    {"name":"check","status":"completed","conclusion":"success"},
    {"name":"generate (keyring)","status":"completed","conclusion":"failure"}]}"#;

/// The cli form exported into `remotes/work/template-cli`, committed and
/// tagged `v1.0.0`, and pushed to `remotes/owner/template-cli.git`.
fn cli_template(remotes: &Path) -> (PathBuf, PathBuf) {
    let template = remotes.join("work/template-cli");
    // Exported for the repository it is followed in: its README names it.
    lyrn()
        .args(["template", "export", "cli", "--repo", "owner/template-cli", "--path"])
        .arg(&template)
        .assert()
        .success();
    git(&template, &["init", "--quiet"]);
    git(&template, &["add", "."]);
    git(&template, &["commit", "--quiet", "-m", "feat: the cli form"]);
    git(&template, &["tag", "-a", "v1.0.0", "-m", "v1.0.0"]);
    fs::create_dir_all(remotes.join("owner")).unwrap();
    let bare = remotes.join("owner/template-cli.git");
    git(remotes, &["clone", "--quiet", "--bare", template.to_str().unwrap(), "owner/template-cli.git"]);
    git(&template, &["remote", "add", "origin", bare.to_str().unwrap()]);
    (template, bare)
}

/// A new commit of the template with `change` made to it, tagged `tag`, and
/// pushed: its commit.
fn release(template: &Path, tag: &str, change: impl FnOnce(&Path)) -> String {
    change(template);
    git(template, &["add", "-A"]);
    git(template, &["commit", "--quiet", "-m", &format!("feat: {tag}")]);
    git(template, &["tag", "-a", tag, "-m", tag]);
    git(template, &["push", "--quiet", "origin", "HEAD:main", tag]);
    git(template, &["rev-parse", "HEAD"])
}

fn append(path: impl AsRef<Path>, line: &str) {
    let text = read(&path);
    fs::write(path, format!("{text}{line}\n")).unwrap();
}

/// A project generated from `owner/template-cli@v1.0.0`, with a line of its
/// own on top of a file of the standard, committed.
fn project_from_the_template(identity: &Identity, remotes: &Path, api: &str) -> PathBuf {
    let out = remotes.join("demo-app");
    identity
        .lyrn(remotes, api)
        .args([
            "new",
            "demo-app",
            "--yes",
            "--no-hooks",
            "--repo",
            "owner/demo-app",
            "--template",
            "owner/template-cli@v1.0.0",
            "--path",
        ])
        .arg(&out)
        .assert()
        .success();
    let editorconfig = read(out.join(".editorconfig"));
    fs::write(out.join(".editorconfig"), format!("# the project's own\n{editorconfig}")).unwrap();
    git(&out, &["init", "--quiet"]);
    git(&out, &["remote", "add", "origin", "https://github.com/owner/demo-app.git"]);
    git(&out, &["add", "-A"]);
    git(&out, &["commit", "--quiet", "-m", "feat: start"]);
    out
}

/// The whole of the promise: a project made from a template takes what the
/// template changed in a file of the standard since, from the newest tag
/// whose CI passed - its own line kept, the template's code not brought in
/// over the project's, and a newer tag whose CI is red passed over by name.
#[test]
fn a_project_from_a_template_takes_its_newest_green_tag() {
    let identity = Identity::new();
    let remotes = tempfile::tempdir().unwrap();
    let (template, _) = cli_template(remotes.path());
    let green = release(&template, "v1.1.0", |t| {
        append(t.join("template/.editorconfig"), "# brought by v1.1.0");
        append(t.join("template/src/main.rs"), "// the template's code at v1.1.0");
    });
    let red = release(&template, "v1.2.0", |t| append(t.join("template/.editorconfig"), "# brought by v1.2.0"));
    let api = api_answering(move |request| if request.contains(&red) { RED.to_string() } else { GREEN.to_string() });
    let out = project_from_the_template(&identity, remotes.path(), &api);

    let assert = identity.lyrn(remotes.path(), &api).args(["upgrade", "--yes"]).arg(&out).assert().success();
    let report = String::from_utf8_lossy(&assert.get_output().stdout).into_owned();
    assert!(report.contains("from the template owner/template-cli@v1.0.0 to v1.1.0"), "{report}");
    assert!(report.contains("v1.2.0 passed over: its CI failed: generate (keyring) (failure)"), "{report}");

    let editorconfig = read(out.join(".editorconfig"));
    assert!(editorconfig.starts_with("# the project's own\n"), "{editorconfig}");
    assert!(editorconfig.contains("# brought by v1.1.0"), "{editorconfig}");
    assert!(!editorconfig.contains("v1.2.0"), "a tag whose CI is red came in: {editorconfig}");
    assert!(
        !read(out.join("src/main.rs")).contains("v1.1.0"),
        "the template's code was merged over the project's"
    );
    let stamp = read(out.join("lyrn.toml"));
    assert!(stamp.contains("source = \"owner/template-cli@v1.1.0\""), "{stamp}");
    assert!(stamp.contains(&format!("revision = \"{green}\"")), "{stamp}");
    assert_eq!(
        git(&out, &["log", "-1", "--format=%s"]),
        "chore(standard): take the template owner/template-cli@v1.1.0"
    );
    assert_eq!(git(&out, &["status", "--porcelain"]), "", "the upgrade left files uncommitted");

    // On the newest green tag, there is nothing to do.
    identity
        .lyrn(remotes.path(), &api)
        .args(["upgrade", "--yes"])
        .arg(&out)
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "is on owner/template-cli@v1.1.0; no newer tag of it passed its CI: nothing to do",
        ));
}

/// Where the project and the template changed the same lines, the markers
/// name the two tags, and nothing is committed.
#[test]
fn lines_a_project_and_its_template_both_changed_are_marked_with_the_tags() {
    let identity = Identity::new();
    let remotes = tempfile::tempdir().unwrap();
    let (template, _) = cli_template(remotes.path());
    let first = read(template.join("template/.editorconfig")).lines().next().unwrap().to_string();
    release(&template, "v1.1.0", |t| {
        edit(t.join("template/.editorconfig"), &first, "# the template's first line")
    });
    let api = api_answering(|_| GREEN.to_string());
    let out = project_from_the_template(&identity, remotes.path(), &api);
    // The project's own line went on top; now it changes the template's first line too.
    edit(
        out.join(".editorconfig"),
        &format!("# the project's own\n{first}"),
        "# the project's first line",
    );
    git(&out, &["commit", "--quiet", "-am", "chore: mine"]);

    identity.lyrn(remotes.path(), &api).args(["upgrade", "--yes"]).arg(&out).assert().failure();
    let marked = read(out.join(".editorconfig"));
    for marker in ["<<<<<<< yours", "||||||| owner/template-cli@v1.0.0", ">>>>>>> owner/template-cli@v1.1.0"] {
        assert!(marked.lines().any(|l| l == marker), "no `{marker}` in:\n{marked}");
    }
    assert_eq!(git(&out, &["log", "-1", "--format=%s"]), "chore: mine");
}

#[test]
fn a_project_from_a_template_directory_is_refused_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let template = exported("cli", dir.path());
    let out = dir.path().join("demo-app");
    lyrn()
        .args(["new", "demo-app", "--yes", "--no-hooks", "--repo", "owner/demo-app", "--template"])
        .arg(&template)
        .arg("--path")
        .arg(&out)
        .assert()
        .success();
    lyrn().arg("upgrade").arg(&out).assert().failure().stderr(predicate::str::contains(
        "came from the template `path:template-cli`, which keeps no revision to upgrade from",
    ));
}
