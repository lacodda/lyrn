//! A template from a GitHub repository, at a tag whose CI passed.
//!
//! The order is what makes the check mean something. The tag is resolved to a
//! commit first; the CI is asked about that commit, not about the tag's name;
//! and the clone is held to the same commit. A tag moved between the question
//! and the clone is a refusal, so the files a project is generated from are
//! always the files the CI built.

use std::error::Error;
use std::path::Path;
use std::process::Command as Process;
use std::time::Duration;

use serde_json::Value;

use super::{Origin, Template, load_dir};

/// Where repositories are cloned from.
const GITHUB: &str = "https://github.com";
/// Where the CI of a commit is asked about.
const GITHUB_API: &str = "https://api.github.com";

/// The base both are read from. The variables exist for the tests, which
/// stand a local repository and a local API in for GitHub's; nothing else
/// should need them.
fn git_base() -> String {
    std::env::var("LYRN_GITHUB_GIT").unwrap_or_else(|_| GITHUB.to_string())
}

fn api_base() -> String {
    std::env::var("LYRN_GITHUB_API").unwrap_or_else(|_| GITHUB_API.to_string())
}

/// Fetch `owner/repo` at `tag`, if the tag's commit passed its CI.
pub fn fetch(owner: &str, repo: &str, tag: &str) -> Result<Template, Box<dyn Error>> {
    let name = format!("{owner}/{repo}");
    let url = format!("{}/{owner}/{repo}.git", git_base().trim_end_matches('/'));

    let tags = remote_tags(&url, &name)?;
    let commit = tags.iter().find(|t| t.name == tag).map(|t| t.commit.clone()).ok_or_else(|| {
        let newest: Vec<&str> = tags.iter().take(5).map(|t| t.name.as_str()).collect();
        if newest.is_empty() {
            format!("`{name}` has no tags; a template is used at a tag, so its CI can be asked about exactly those files")
        } else {
            format!("`{name}` has no tag `{tag}` (its newest: {})", newest.join(", "))
        }
    })?;

    ci(&name, &commit)?.require(&name, tag, &commit)?;

    let checkout = tempfile::Builder::new().prefix("lyrn-template-").tempdir()?;
    let dir = checkout.path().join("checkout");
    clone(&url, tag, &dir)?;
    let cloned = git_output(&dir, &["rev-parse", "HEAD"])?;
    if cloned != commit {
        return Err(format!(
            "the tag `{tag}` of `{name}` moved while lyrn was reading it ({} became {}); nothing was generated - run again",
            short(&commit),
            short(&cloned)
        )
        .into());
    }

    // Read whole into memory here: the checkout is gone when this returns.
    load_dir(
        &dir,
        Origin::GitHub {
            repo: name,
            tag: tag.to_string(),
            commit,
        },
    )
}

fn short(commit: &str) -> &str {
    &commit[..commit.len().min(12)]
}

/// A tag and the commit it points at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteTag {
    pub name: String,
    pub commit: String,
}

fn remote_tags(url: &str, name: &str) -> Result<Vec<RemoteTag>, Box<dyn Error>> {
    let output = git()
        .args(["ls-remote", "--tags", "--sort=-version:refname", url])
        .output()
        .map_err(|e| format!("lyrn reads templates from GitHub with git, which did not start: {e}"))?;
    if !output.status.success() {
        return Err(format!("could not read `{name}` from GitHub: {}", String::from_utf8_lossy(&output.stderr).trim()).into());
    }
    Ok(parse_tags(&String::from_utf8_lossy(&output.stdout)))
}

/// Read `git ls-remote --tags`: one `<commit>\trefs/tags/<name>` per line,
/// and for an annotated tag a second, `^{}`, line with the commit it points
/// at. The peeled line wins - the first names the tag object, which no CI
/// has ever run on.
pub fn parse_tags(listing: &str) -> Vec<RemoteTag> {
    let mut tags: Vec<RemoteTag> = Vec::new();
    for line in listing.lines() {
        let Some((commit, reference)) = line.split_once('\t') else { continue };
        let Some(name) = reference.strip_prefix("refs/tags/") else { continue };
        match name.strip_suffix("^{}") {
            Some(name) => match tags.iter_mut().find(|t| t.name == name) {
                Some(tag) => tag.commit = commit.to_string(),
                None => tags.push(RemoteTag {
                    name: name.to_string(),
                    commit: commit.to_string(),
                }),
            },
            None if tags.iter().any(|t| t.name == name) => {}
            None => tags.push(RemoteTag {
                name: name.to_string(),
                commit: commit.to_string(),
            }),
        }
    }
    tags
}

fn clone(url: &str, tag: &str, dir: &Path) -> Result<(), Box<dyn Error>> {
    // Line endings as committed: a template's bytes are what its CI built,
    // and a checkout that turns LF into CRLF generates something else.
    let output = git()
        .args([
            "-c",
            "core.autocrlf=false",
            "-c",
            "advice.detachedHead=false",
            "clone",
            "--quiet",
            "--depth",
            "1",
            "--branch",
            tag,
            url,
        ])
        .arg(dir)
        .output()?;
    if !output.status.success() {
        return Err(format!("could not clone `{url}` at `{tag}`: {}", String::from_utf8_lossy(&output.stderr).trim()).into());
    }
    Ok(())
}

fn git() -> Process {
    let mut command = Process::new("git");
    // A repository that is private or missing makes git ask for a password,
    // and a prompt nobody is going to answer hangs the run.
    command.env("GIT_TERMINAL_PROMPT", "0");
    command
}

fn git_output(dir: &Path, args: &[&str]) -> Result<String, Box<dyn Error>> {
    let output = git().args(args).current_dir(dir).output()?;
    if !output.status.success() {
        return Err(format!("`git {}` failed: {}", args.join(" "), String::from_utf8_lossy(&output.stderr).trim()).into());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// What the CI said about a commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Something ran, it all finished, and nothing failed.
    Green,
    /// Nothing ran on the commit, or all of it was skipped.
    NoCi,
    /// Still running: the names of what has not finished.
    Running(Vec<String>),
    /// Finished and failed: each name with how it ended.
    Failed(Vec<String>),
}

impl Verdict {
    fn require(self, name: &str, tag: &str, commit: &str) -> Result<(), String> {
        // The way round a refusal is named, and it is not a flag: a template
        // nobody's CI has built can still be used, by someone who clones it
        // and so takes the files on their own judgement.
        let own_risk = format!(
            "to use it anyway, clone it and pass the directory: `--template ./{}`",
            name.split('/').nth(1).unwrap_or(name)
        );
        match self {
            Verdict::Green => Ok(()),
            Verdict::NoCi => Err(format!(
                "`{name}@{tag}` has no CI result on its commit {}; lyrn uses a template only once its own CI has generated and built it - {own_risk}",
                short(commit)
            )),
            Verdict::Running(names) => Err(format!(
                "the CI of `{name}@{tag}` is still running ({}); try again once it finishes",
                names.join(", ")
            )),
            Verdict::Failed(names) => Err(format!(
                "the CI of `{name}@{tag}` failed: {}; lyrn does not generate from a template whose CI is red - {own_risk}",
                names.join(", ")
            )),
        }
    }
}

/// Weigh a commit's check runs and its commit statuses together: a repository
/// may report through either, and both count.
pub fn verdict(check_runs: &[Value], statuses: &[Value]) -> Verdict {
    let mut running = Vec::new();
    let mut failed = Vec::new();
    let mut passed = 0;

    for run in check_runs {
        let name = run["name"].as_str().unwrap_or("a check").to_string();
        if run["status"].as_str() != Some("completed") {
            running.push(name);
            continue;
        }
        match run["conclusion"].as_str() {
            Some("success") => passed += 1,
            // Neither counts as proof, and neither is a failure: a skipped job
            // is one whose condition did not hold.
            Some("neutral") | Some("skipped") => {}
            other => failed.push(format!("{name} ({})", other.unwrap_or("no conclusion"))),
        }
    }
    for status in statuses {
        let name = status["context"].as_str().unwrap_or("a status").to_string();
        match status["state"].as_str() {
            Some("success") => passed += 1,
            Some("pending") => running.push(name),
            other => failed.push(format!("{name} ({})", other.unwrap_or("no state"))),
        }
    }

    if !failed.is_empty() {
        Verdict::Failed(failed)
    } else if !running.is_empty() {
        Verdict::Running(running)
    } else if passed == 0 {
        Verdict::NoCi
    } else {
        Verdict::Green
    }
}

/// Ask GitHub what the CI said about `commit`.
fn ci(name: &str, commit: &str) -> Result<Verdict, Box<dyn Error>> {
    let client = reqwest::blocking::Client::builder()
        .user_agent(concat!("lyrn/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(30))
        .build()?;
    let base = api_base();
    // The token goes to GitHub and nowhere else: a base pointed elsewhere is a
    // test's, and a token has no business leaving for it.
    let token = if base == GITHUB_API { token() } else { None };

    let get = |path: &str| -> Result<Value, Box<dyn Error>> {
        let mut request = client
            .get(format!("{base}/repos/{name}/commits/{commit}/{path}"))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28");
        if let Some(token) = &token {
            request = request.bearer_auth(token);
        }
        let response = request.send().map_err(|e| format!("could not ask GitHub about the CI of `{name}`: {e}"))?;
        let status = response.status();
        if status.is_success() {
            return Ok(serde_json::from_str(&response.text()?)?);
        }
        let spent = response.headers().get("x-ratelimit-remaining").is_some_and(|v| v == "0");
        Err(match status.as_u16() {
            403 | 429 if spent => "GitHub's limit on anonymous questions is spent for the hour; `gh auth login` or GH_TOKEN lifts it".to_string(),
            404 if token.is_none() => format!("GitHub does not show the CI of `{name}`; if it is private, `gh auth login` or GH_TOKEN lets lyrn ask"),
            _ => format!("GitHub answered {status} when asked about the CI of `{name}`"),
        }
        .into())
    };

    let mut check_runs = Vec::new();
    for page in 1..=10 {
        let body = get(&format!("check-runs?per_page=100&page={page}"))?;
        let runs = body["check_runs"].as_array().cloned().unwrap_or_default();
        let total = body["total_count"].as_u64().unwrap_or(0) as usize;
        let empty = runs.is_empty();
        check_runs.extend(runs);
        if empty || check_runs.len() >= total {
            break;
        }
    }
    let combined = get("status")?;
    let statuses = combined["statuses"].as_array().cloned().unwrap_or_default();

    Ok(verdict(&check_runs, &statuses))
}

/// A GitHub token, if one is at hand: the environment first, then the one
/// `gh` is logged in with. Without one lyrn asks anonymously, which is enough
/// for a public template and sixty questions an hour.
fn token() -> Option<String> {
    for var in ["GH_TOKEN", "GITHUB_TOKEN"] {
        if let Some(value) = std::env::var(var).ok().filter(|v| !v.trim().is_empty()) {
            return Some(value.trim().to_string());
        }
    }
    let output = Process::new("gh").args(["auth", "token"]).output().ok()?;
    let token = String::from_utf8(output.stdout).ok()?.trim().to_string();
    (output.status.success() && !token.is_empty()).then_some(token)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn run(name: &str, status: &str, conclusion: Option<&str>) -> Value {
        json!({ "name": name, "status": status, "conclusion": conclusion })
    }

    #[test]
    fn everything_passed_is_green() {
        let runs = [run("build", "completed", Some("success")), run("lint", "completed", Some("skipped"))];
        assert_eq!(verdict(&runs, &[]), Verdict::Green);
    }

    #[test]
    fn nothing_at_all_is_no_ci() {
        assert_eq!(verdict(&[], &[]), Verdict::NoCi);
    }

    /// A commit whose every job was skipped has been built by nobody.
    #[test]
    fn nothing_but_skips_is_no_ci() {
        assert_eq!(verdict(&[run("build", "completed", Some("skipped"))], &[]), Verdict::NoCi);
    }

    #[test]
    fn one_failure_among_successes_is_a_failure() {
        let runs = [run("linux", "completed", Some("success")), run("windows", "completed", Some("failure"))];
        assert_eq!(verdict(&runs, &[]), Verdict::Failed(vec!["windows (failure)".into()]));
    }

    #[test]
    fn a_cancelled_or_timed_out_run_is_a_failure() {
        for conclusion in ["cancelled", "timed_out", "action_required", "stale"] {
            let runs = [run("build", "completed", Some("success")), run("test", "completed", Some(conclusion))];
            assert!(matches!(verdict(&runs, &[]), Verdict::Failed(_)), "{conclusion} passed");
        }
    }

    #[test]
    fn an_unfinished_run_is_running() {
        let runs = [run("build", "completed", Some("success")), run("test", "in_progress", None)];
        assert_eq!(verdict(&runs, &[]), Verdict::Running(vec!["test".into()]));
    }

    #[test]
    fn commit_statuses_count_as_well() {
        let ok = json!({ "context": "ci/legacy", "state": "success" });
        let bad = json!({ "context": "ci/legacy", "state": "failure" });
        let pending = json!({ "context": "ci/legacy", "state": "pending" });
        assert_eq!(verdict(&[], std::slice::from_ref(&ok)), Verdict::Green);
        assert!(matches!(verdict(&[], &[bad]), Verdict::Failed(_)));
        assert!(matches!(verdict(&[run("b", "completed", Some("success"))], &[pending]), Verdict::Running(_)));
    }

    #[test]
    fn an_annotated_tag_names_the_commit_it_points_at() {
        let listing = "aaa\trefs/tags/v2.0.0\nbbb\trefs/tags/v1.0.0\nccc\trefs/tags/v1.0.0^{}\n";
        assert_eq!(
            parse_tags(listing),
            vec![
                RemoteTag {
                    name: "v2.0.0".into(),
                    commit: "aaa".into()
                },
                RemoteTag {
                    name: "v1.0.0".into(),
                    commit: "ccc".into()
                },
            ]
        );
    }

    #[test]
    fn the_peeled_line_wins_in_either_order() {
        let listing = "ccc\trefs/tags/v1.0.0^{}\nbbb\trefs/tags/v1.0.0\n";
        assert_eq!(parse_tags(listing)[0].commit, "ccc");
    }

    #[test]
    fn a_refusal_names_the_way_round_it() {
        let err = Verdict::NoCi.require("someone/template-x", "v1", "0123456789abcdef").unwrap_err();
        assert!(err.contains("--template ./template-x"), "{err}");
    }
}
