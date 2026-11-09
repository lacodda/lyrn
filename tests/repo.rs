//! `lyrn repo` against a GitHub of the test's own: a server that answers the
//! API from a state it keeps and writes down every request, so the test can
//! hold the command to what it asked GitHub to do - and to asking nothing to
//! be written on a dry run.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::process::Command as Process;
use std::sync::{Arc, Mutex};

use serde_json::{Value, json};

mod common;

/// One request the server was sent.
#[derive(Debug, Clone)]
struct Request {
    method: String,
    path: String,
    body: Value,
}

/// What the stand-in GitHub holds.
struct GitHub {
    repo: Option<Value>,
    pages: Option<Value>,
    reporting: bool,
    rulesets: Vec<Value>,
    requests: Vec<Request>,
}

impl GitHub {
    fn answer(&mut self, method: &str, path: &str, body: &Value) -> (u16, Value) {
        let path = path.split('?').next().unwrap_or(path);
        match (method, path) {
            ("GET", "/user") => (200, json!({ "login": "owner" })),
            ("GET", "/repos/owner/demo") => match &self.repo {
                Some(repo) => (200, repo.clone()),
                None => (404, json!({ "message": "Not Found" })),
            },
            ("POST", "/user/repos") => {
                self.repo = Some(json!({
                    "visibility": if body["private"] == true { "private" } else { "public" },
                    "default_branch": "main",
                    "description": body["description"],
                    "homepage": body["homepage"],
                    "topics": [],
                }));
                (201, self.repo.clone().unwrap())
            }
            ("PATCH", "/repos/owner/demo") => {
                let repo = self.repo.as_mut().unwrap();
                for (key, value) in body.as_object().unwrap() {
                    repo[key] = value.clone();
                }
                (200, repo.clone())
            }
            ("PUT", "/repos/owner/demo/topics") => {
                self.repo.as_mut().unwrap()["topics"] = body["names"].clone();
                (200, json!({ "names": body["names"] }))
            }
            ("GET", "/repos/owner/demo/pages") => match &self.pages {
                Some(pages) => (200, pages.clone()),
                None => (404, json!({ "message": "Not Found" })),
            },
            ("POST", "/repos/owner/demo/pages") => {
                self.pages = Some(json!({ "build_type": body["build_type"], "cname": null }));
                (201, self.pages.clone().unwrap())
            }
            ("PUT", "/repos/owner/demo/pages") => {
                let pages = self.pages.as_mut().unwrap();
                for (key, value) in body.as_object().unwrap() {
                    pages[key] = value.clone();
                }
                (204, Value::Null)
            }
            ("GET", "/repos/owner/demo/private-vulnerability-reporting") => (200, json!({ "enabled": self.reporting })),
            ("PUT", "/repos/owner/demo/private-vulnerability-reporting") => {
                self.reporting = true;
                (204, Value::Null)
            }
            ("GET", "/repos/owner/demo/rulesets") => (
                200,
                Value::Array(self.rulesets.iter().map(|r| json!({ "id": r["id"], "name": r["name"] })).collect()),
            ),
            ("POST", "/repos/owner/demo/rulesets") => {
                let mut ruleset = body.clone();
                ruleset["id"] = json!(self.rulesets.len() + 1);
                self.rulesets.push(ruleset.clone());
                (201, ruleset)
            }
            ("GET", "/repos/owner/demo/actions/workflows/ci.yml/runs") => (200, json!({ "workflow_runs": [{ "id": 42 }] })),
            ("GET", "/repos/owner/demo/actions/runs/42/jobs") => (
                200,
                json!({ "jobs": [{ "name": "ubuntu-latest" }, { "name": "windows-latest" }, { "name": "msrv" }] }),
            ),
            ("POST", "/graphql") => (
                200,
                json!({ "data": { "repository": { "usesCustomOpenGraphImage": false, "openGraphImageUrl": null } } }),
            ),
            (_, path) => {
                if let Some(id) = path.strip_prefix("/repos/owner/demo/rulesets/")
                    && let Some(ruleset) = self.rulesets.iter().find(|r| r["id"].to_string() == id)
                {
                    return (200, ruleset.clone());
                }
                (404, json!({ "message": "Not Found" }))
            }
        }
    }
}

/// A server standing in for GitHub's API, for as long as the test runs.
fn serve(github: Arc<Mutex<GitHub>>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            if reader.read_line(&mut line).is_err() {
                continue;
            }
            let mut parts = line.split_whitespace();
            let method = parts.next().unwrap_or_default().to_string();
            let path = parts.next().unwrap_or("/").to_string();
            let mut length = 0;
            loop {
                let mut header = String::new();
                if reader.read_line(&mut header).is_err() || header.trim().is_empty() {
                    break;
                }
                if let Some((name, value)) = header.split_once(':')
                    && name.eq_ignore_ascii_case("content-length")
                {
                    length = value.trim().parse().unwrap_or(0);
                }
            }
            let mut body = vec![0; length];
            let _ = reader.read_exact(&mut body);
            let body: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
            let (status, answer) = {
                let mut github = github.lock().unwrap();
                github.requests.push(Request {
                    method: method.clone(),
                    path: path.clone(),
                    body: body.clone(),
                });
                github.answer(&method, &path, &body)
            };
            let text = if answer.is_null() { String::new() } else { answer.to_string() };
            let response = format!(
                "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}",
                text.len()
            );
            let _ = stream.write_all(response.as_bytes());
        }
    });
    address
}

fn git(dir: &Path, args: &[&str]) {
    let out = Process::new("git").args(args).current_dir(dir).output().unwrap();
    assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
}

/// A project that says what it is: a description, keywords, the repository
/// it lives in, a site deployed by a workflow to a domain of its own, and
/// the CI its release waits for.
fn project(dir: &Path) {
    std::fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2024\"\ndescription = \"A demo of the line\"\nrepository = \"https://github.com/owner/demo\"\nhomepage = \"https://demo.dev/\"\nkeywords = [\"cli\", \"Command Line\"]\nlicense = \"MIT\"\n",
    )
    .unwrap();
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("src/main.rs"), "fn main() {}\n").unwrap();
    std::fs::write(dir.join("release.toml"), "[ci]\nprovider = \"github\"\nworkflow = \"ci.yml\"\n").unwrap();
    std::fs::create_dir_all(dir.join(".github/workflows")).unwrap();
    std::fs::write(dir.join(".github/workflows/ci.yml"), "name: CI\non: push\njobs: {}\n").unwrap();
    std::fs::write(
        dir.join(".github/workflows/docs.yml"),
        "jobs:\n  deploy:\n    steps:\n      - uses: actions/deploy-pages@v5\n",
    )
    .unwrap();
    std::fs::create_dir_all(dir.join("docs/public")).unwrap();
    std::fs::write(dir.join("docs/public/CNAME"), "demo.dev\n").unwrap();
    std::fs::create_dir_all(dir.join("assets")).unwrap();
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/social-preview.png"),
        dir.join("assets/social-preview.png"),
    )
    .unwrap();
    git(dir, &["init", "--quiet", "--initial-branch", "main"]);
    git(dir, &["add", "."]);
    git(
        dir,
        &[
            "-c",
            "user.name=Ada",
            "-c",
            "user.email=ada@example.com",
            "commit",
            "--quiet",
            "-m",
            "feat: start",
        ],
    );
}

fn lyrn_repo(dir: &Path, api: &str, git_base: &Path, args: &[&str]) -> (String, bool) {
    let out = common::lyrn()
        .arg("repo")
        .args(args)
        .current_dir(dir)
        .env("LYRN_GITHUB_API", api)
        .env("LYRN_GITHUB_GIT", git_base)
        .env("GH_TOKEN", "")
        .env("GITHUB_TOKEN", "")
        .output()
        .unwrap();
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    (text, out.status.success())
}

fn written(github: &Arc<Mutex<GitHub>>) -> Vec<Request> {
    github
        .lock()
        .unwrap()
        .requests
        .iter()
        .filter(|r| r.method != "GET" && r.path != "/graphql")
        .cloned()
        .collect()
}

/// A repository GitHub has never seen is created under the account signed
/// in, pushed to, and given every setting the project says - and a second
/// run finds nothing left to set.
#[test]
fn a_new_repository_is_created_and_kept_the_way_the_project_says() {
    let dir = tempfile::tempdir().unwrap();
    let work = dir.path().join("demo");
    std::fs::create_dir_all(&work).unwrap();
    project(&work);
    let remotes = dir.path().join("remotes");
    std::fs::create_dir_all(remotes.join("owner")).unwrap();
    git(&remotes.join("owner"), &["init", "--quiet", "--bare", "demo.git"]);

    let github = Arc::new(Mutex::new(GitHub {
        repo: None,
        pages: None,
        reporting: false,
        rulesets: Vec::new(),
        requests: Vec::new(),
    }));
    let api = serve(github.clone());

    let (dry, ok) = lyrn_repo(&work, &api, &remotes, &["--dry-run"]);
    assert!(ok, "{dry}");
    assert!(dry.contains("→ repository      create owner/demo, public"), "{dry}");
    assert!(written(&github).is_empty(), "a dry run writes nothing: {:?}", written(&github));

    let (text, ok) = lyrn_repo(&work, &api, &remotes, &[]);
    assert!(ok, "{text}");
    let requests = written(&github);
    let sent = |method: &str, path: &str| {
        requests
            .iter()
            .filter(|r| r.method == method && r.path == path)
            .map(|r| r.body.clone())
            .collect::<Vec<_>>()
    };

    let created = sent("POST", "/user/repos");
    assert_eq!(created.len(), 1, "{text}");
    assert_eq!(created[0]["name"], "demo");
    assert_eq!(created[0]["private"], false);
    assert_eq!(created[0]["description"], "A demo of the line");
    let pushed = Process::new("git")
        .args(["rev-parse", "main"])
        .current_dir(remotes.join("owner/demo.git"))
        .output()
        .unwrap();
    assert!(pushed.status.success(), "what is committed is pushed to the new repository");

    assert_eq!(sent("PUT", "/repos/owner/demo/topics"), [json!({ "names": ["cli", "command-line"] })]);
    assert_eq!(sent("POST", "/repos/owner/demo/pages"), [json!({ "build_type": "workflow" })]);
    assert_eq!(sent("PUT", "/repos/owner/demo/pages"), [json!({ "cname": "demo.dev" })]);
    assert_eq!(sent("PUT", "/repos/owner/demo/private-vulnerability-reporting").len(), 1);

    let rulesets = sent("POST", "/repos/owner/demo/rulesets");
    assert_eq!(rulesets.len(), 2, "{text}");
    let tags = rulesets.iter().find(|r| r["target"] == "tag").unwrap();
    assert_eq!(tags["conditions"]["ref_name"]["include"], json!(["refs/tags/v*"]));
    let required = tags["rules"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["type"] == "required_status_checks")
        .unwrap();
    let checks: Vec<&str> = required["parameters"]["required_status_checks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["context"].as_str().unwrap())
        .collect();
    assert_eq!(checks, ["msrv", "ubuntu-latest", "windows-latest"]);
    let branch = rulesets.iter().find(|r| r["target"] == "branch").unwrap();
    assert!(
        branch["rules"].as_array().unwrap().iter().all(|r| r["type"] != "required_status_checks"),
        "a push to main is not refused"
    );
    assert!(text.contains("! social preview"), "{text}");

    let before = written(&github).len();
    let (again, ok) = lyrn_repo(&work, &api, &remotes, &[]);
    assert!(ok, "{again}");
    assert_eq!(written(&github).len(), before, "nothing is left to set: {again}");
    assert!(again.contains("8 already so"), "{again}");
}
