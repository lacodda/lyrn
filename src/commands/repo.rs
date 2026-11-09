//! `lyrn repo`: the repository on GitHub, kept the way the line keeps them.
//!
//! A repository on GitHub carries settings no file of it holds: what it says
//! it is, where its home is, its topics, how its pages are built, whether a
//! vulnerability can be reported in private, which refs cannot be rewritten.
//! Each was set by hand once, in a settings page, and drifted from the
//! project the day the project changed. Every one of them is something the
//! project already says - in its manifests, its workflows, its SECURITY.md,
//! its release - so the command reads them from there and makes GitHub say
//! the same, creating the repository first when there is none.
//!
//! What holds the release to green CI is a rule on GitHub rather than a habit:
//! a version tag can only be created on a commit whose CI passed, and never
//! moved or deleted after. The default branch keeps its history the same way.
//! The checks are not required of the branch itself: the line pushes to `main`
//! and lets CI judge the push, and a branch that refuses an unchecked commit
//! refuses every push.

use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command as Process;

use serde_json::{Value, json};

use crate::cli::RepoArgs;
use crate::commands::assets::SOCIAL;
use crate::commands::check::product::Product;
use crate::commands::check::site::Site;
use crate::commands::doctor::{Repo, is_test_material};
use crate::github::{Api, Refusal};

/// The rule on the version tags, by the name GitHub lists it under.
pub const TAG_RULES: &str = "Release tags";
/// The rule on the default branch.
pub const BRANCH_RULES: &str = "Default branch";
/// The tags a release is: `v1.2.3`, and only those.
const TAGS: &str = "refs/tags/v*";
/// The most topics GitHub keeps on a repository.
const MOST_TOPICS: usize = 20;

pub fn run(args: RepoArgs) -> Result<bool, Box<dyn Error>> {
    let path = args.path.clone().unwrap_or_else(|| PathBuf::from("."));
    if !path.is_dir() {
        return Err(format!("`{}` is not a directory - `lyrn repo` keeps a repository", path.display()).into());
    }
    let repo = Repo::read(&path);
    if repo.git.is_none() {
        return Err(format!("`{}` is not in a git repository - `lyrn repo` puts one on GitHub", path.display()).into());
    }
    let product = Product::read(&repo);
    let slug = product.github.clone().ok_or(
        "no GitHub repository is named here: `origin` points at none, and no manifest says `repository = \"https://github.com/OWNER/NAME\"` - name it in the manifest, or add the remote",
    )?;
    let wanted = Wanted::read(&repo, &product);
    let api = Api::new()?;
    if !api.signed_in() {
        return Err("`lyrn repo` writes to GitHub, and there is no token to write with: `gh auth login`, or GH_TOKEN".into());
    }

    println!("lyrn repo: {}, {slug}\n", repo.root.display());
    let mut report = Report::new(args.dry_run);

    let current = match api.find(&format!("repos/{slug}"))? {
        Some(current) => {
            let visibility = current["visibility"].as_str().unwrap_or("public").to_string();
            report.holds("repository", format!("{visibility}, on GitHub"));
            current
        }
        None if args.dry_run => {
            let visibility = if args.private { "private" } else { "public" };
            report.change("repository", format!("create {slug}, {visibility}, and push what is committed"));
            report.later(
                "the settings below",
                "once the repository is there: the rest follows on `lyrn repo` without --dry-run",
            );
            report.print();
            return Ok(true);
        }
        None => {
            create(&api, &slug, &wanted, args.private)?;
            let created = api.get(&format!("repos/{slug}"))?;
            let visibility = created["visibility"].as_str().unwrap_or("public").to_string();
            report.change("repository", format!("created, {visibility}{}", push_origin(&repo.root, &slug)?));
            created
        }
    };
    let public = current["visibility"].as_str() != Some("private");

    settle_about(&api, &slug, &wanted, &current, &mut report);
    settle_topics(&api, &slug, &wanted, &current, &mut report);
    settle_pages(&api, &slug, &wanted, &mut report);
    settle_reports(&api, &slug, public, &mut report);
    let branch = current["default_branch"].as_str().unwrap_or("main").to_string();
    settle_rules(&api, &slug, &wanted, &branch, &mut report);
    settle_preview(&api, &slug, &repo.root, &mut report);

    report.print();
    Ok(!report.failed())
}

/// What the project says about itself, as GitHub should repeat it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Wanted {
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub topics: Vec<String>,
    /// Where the topics come from, for the report.
    pub topics_from: Vec<String>,
    /// The pages a workflow deploys, if one does: the domain they are served
    /// at, when `public/CNAME` names one.
    pub pages: Option<Option<String>>,
    /// The workflow a release waits for: `[ci] workflow` in release.toml.
    pub ci: Option<String>,
}

impl Wanted {
    pub fn read(repo: &Repo, product: &Product) -> Self {
        // The top manifests speak first: they are what the registries show.
        let mut manifests: Vec<_> = product.manifests.iter().collect();
        manifests.sort_by_key(|m| (!m.is_root(), m.path.matches('/').count()));
        let description = manifests
            .iter()
            .find_map(|m| m.description.clone().filter(|d| !d.trim().is_empty()))
            .map(|d| crate::naming::one_line(&d));
        let site = Site::read(repo);
        let homepage = manifests
            .iter()
            .find_map(|m| m.homepage.clone().filter(|h| !h.trim().is_empty()))
            .or_else(|| site.as_ref().and_then(site_address));
        let mut topics = Vec::new();
        let mut topics_from = Vec::new();
        for manifest in manifests.iter().filter(|m| m.publishable || m.is_root()) {
            let before = topics.len();
            for keyword in &manifest.keywords {
                if let Some(topic) = topic(keyword)
                    && !topics.contains(&topic)
                    && topics.len() < MOST_TOPICS
                {
                    topics.push(topic);
                }
            }
            if topics.len() > before {
                topics_from.push(manifest.path.clone());
            }
        }
        let deploys_pages = repo
            .files
            .iter()
            .filter(|f| f.starts_with(".github/workflows/") && (f.ends_with(".yml") || f.ends_with(".yaml")))
            .filter_map(|f| repo.read_text(f))
            .any(|text| text.contains("actions/deploy-pages"));
        let pages = deploys_pages.then(|| {
            repo.files
                .iter()
                .filter(|f| (*f == "public/CNAME" || f.ends_with("/public/CNAME")) && !is_test_material(f))
                .min_by_key(|f| f.matches('/').count())
                .and_then(|f| repo.read_text(f))
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
        });
        let ci = release_workflow(&repo.root).or_else(|| repo.files.iter().any(|f| f == ".github/workflows/ci.yml").then(|| "ci.yml".to_string()));
        Self {
            description,
            homepage,
            topics,
            topics_from,
            pages,
            ci,
        }
    }
}

/// The address the site is served at, as its config builds it.
pub fn site_address(site: &Site) -> Option<String> {
    let address = site.site.as_deref()?.trim_end_matches('/');
    let base = site.base.as_deref().map(|b| b.trim_matches('/')).filter(|b| !b.is_empty());
    Some(match base {
        Some(base) => format!("{address}/{base}/"),
        None => format!("{address}/"),
    })
}

/// `[ci] workflow` in release.toml: the workflow the release waits for.
pub fn release_workflow(root: &Path) -> Option<String> {
    let text = std::fs::read_to_string(root.join("release.toml")).ok()?;
    let table: toml::Table = toml::from_str(&text).ok()?;
    table.get("ci")?.get("workflow")?.as_str().map(str::to_string)
}

/// A keyword as a GitHub topic: lowercase letters, digits and hyphens, at
/// most 50, starting with a letter or a digit.
pub fn topic(keyword: &str) -> Option<String> {
    let mut out = String::new();
    for c in keyword.trim().chars() {
        match c {
            'a'..='z' | '0'..='9' => out.push(c),
            'A'..='Z' => out.push(c.to_ascii_lowercase()),
            ' ' | '_' | '-' | '.' if !out.is_empty() && !out.ends_with('-') => out.push('-'),
            _ => {}
        }
    }
    let out = out.trim_end_matches('-').to_string();
    (!out.is_empty() && out.len() <= 50).then_some(out)
}

/// Two addresses are one when they differ by a trailing slash alone.
fn same_address(a: &str, b: &str) -> bool {
    a.trim_end_matches('/') == b.trim_end_matches('/')
}

/// What a line of the report says happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    /// GitHub already says it.
    Holds,
    /// Written now, or to be written without --dry-run.
    Changed,
    /// Left to a hand: GitHub has no API for it, or the plan does not have it.
    Hand,
    /// Nothing to settle here.
    Skip,
    /// GitHub refused.
    Failed,
}

struct Line {
    state: State,
    name: String,
    text: String,
    fix: Option<String>,
}

/// The report, line by line, in the order the settings are settled.
struct Report {
    dry_run: bool,
    lines: Vec<Line>,
}

impl Report {
    fn new(dry_run: bool) -> Self {
        Self { dry_run, lines: Vec::new() }
    }

    fn push(&mut self, state: State, name: &str, text: String, fix: Option<String>) {
        self.lines.push(Line {
            state,
            name: name.to_string(),
            text,
            fix,
        });
    }

    fn holds(&mut self, name: &str, text: String) {
        self.push(State::Holds, name, text, None);
    }

    fn change(&mut self, name: &str, text: String) {
        self.push(State::Changed, name, text, None);
    }

    fn hand(&mut self, name: &str, text: String, fix: String) {
        self.push(State::Hand, name, text, Some(fix));
    }

    fn skip(&mut self, name: &str, text: String) {
        self.push(State::Skip, name, text, None);
    }

    fn later(&mut self, name: &str, text: &str) {
        self.push(State::Skip, name, text.to_string(), None);
    }

    fn fail(&mut self, name: &str, refusal: &Refusal) {
        if refusal.is_the_plan() {
            self.push(
                State::Hand,
                name,
                "not on this account's plan for a private repository".to_string(),
                Some(refusal.message.clone()),
            );
        } else {
            self.push(State::Failed, name, refusal.to_string(), None);
        }
    }

    /// A setting to write: written, unless this is a dry run.
    fn write(&mut self, name: &str, text: String, write: impl FnOnce() -> Result<Value, Refusal>) {
        if self.dry_run {
            self.change(name, text);
            return;
        }
        match write() {
            Ok(_) => self.change(name, text),
            Err(refusal) => self.fail(name, &refusal),
        }
    }

    fn failed(&self) -> bool {
        self.lines.iter().any(|l| l.state == State::Failed)
    }

    fn print(&self) {
        const COLUMN: usize = 16;
        for line in &self.lines {
            let sign = match line.state {
                State::Holds => '✓',
                State::Changed => '→',
                State::Hand => '!',
                State::Skip => '-',
                State::Failed => '✗',
            };
            println!("{sign} {:<COLUMN$}{}", line.name, line.text);
            if let Some(fix) = &line.fix {
                println!("    → {fix}");
            }
        }
        let count = |state: State| self.lines.iter().filter(|l| l.state == state).count();
        let mut parts = Vec::new();
        let changed = count(State::Changed);
        if changed > 0 {
            parts.push(if self.dry_run {
                format!("{changed} to set")
            } else {
                format!("{changed} set")
            });
        }
        parts.push(format!("{} already so", count(State::Holds)));
        let hand = count(State::Hand);
        if hand > 0 {
            parts.push(format!("{hand} by hand"));
        }
        let failed = count(State::Failed);
        if failed > 0 {
            parts.push(format!("{failed} refused"));
        }
        println!("\n{}.", parts.join(", "));
        if self.dry_run && changed > 0 {
            println!("Nothing was written: `lyrn repo` without --dry-run sets them.");
        }
    }
}

/// The repository created on GitHub, under its owner: the account signed in,
/// or an organisation it belongs to.
fn create(api: &Api, slug: &str, wanted: &Wanted, private: bool) -> Result<(), Box<dyn Error>> {
    let (owner, name) = slug.split_once('/').ok_or_else(|| format!("`{slug}` is not OWNER/NAME"))?;
    let login = api.get("user")?["login"].as_str().unwrap_or_default().to_string();
    let path = if login.eq_ignore_ascii_case(owner) {
        "user/repos".to_string()
    } else {
        format!("orgs/{owner}/repos")
    };
    let mut body = json!({ "name": name, "private": private });
    if let Some(description) = &wanted.description {
        body["description"] = json!(description);
    }
    if let Some(homepage) = &wanted.homepage {
        body["homepage"] = json!(homepage);
    }
    api.send("POST", &path, Some(&body))
        .map_err(|refusal| format!("{slug} could not be created: {refusal}"))?;
    Ok(())
}

/// `origin` pointed at the new repository and what is committed pushed to
/// it; the words for the report.
fn push_origin(root: &Path, slug: &str) -> Result<String, Box<dyn Error>> {
    let git = |args: &[&str]| Process::new("git").args(args).current_dir(root).output();
    let mut said = String::new();
    let has_origin = git(&["remote", "get-url", "origin"])?.status.success();
    if !has_origin {
        let url = format!("{}/{slug}.git", crate::template::github::git_base().trim_end_matches('/'));
        let out = git(&["remote", "add", "origin", &url])?;
        if !out.status.success() {
            return Err(format!("`git remote add origin {url}` failed: {}", String::from_utf8_lossy(&out.stderr).trim()).into());
        }
        said.push_str(", origin added");
    }
    if !git(&["rev-parse", "--verify", "--quiet", "HEAD"])?.status.success() {
        said.push_str(", nothing committed to push yet");
        return Ok(said);
    }
    let out = git(&["push", "--quiet", "-u", "origin", "HEAD"])?;
    if !out.status.success() {
        return Err(format!(
            "the repository is on GitHub, and the push to it failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )
        .into());
    }
    said.push_str(", what is committed pushed");
    Ok(said)
}

/// The description and the homepage, as the manifests and the site say them.
fn settle_about(api: &Api, slug: &str, wanted: &Wanted, current: &Value, report: &mut Report) {
    let had = |key: &str| current[key].as_str().filter(|s| !s.is_empty()).map(str::to_string);
    match &wanted.description {
        None => report.skip("description", "no manifest says what the project is".to_string()),
        Some(description) if had("description").as_deref() == Some(description.as_str()) => {
            report.holds("description", format!("the manifests': {description}"));
        }
        Some(description) => {
            let body = json!({ "description": description });
            let was = had("description").map(|d| format!("; was: {d}")).unwrap_or_default();
            report.write("description", format!("the manifests': {description}{was}"), || {
                api.send("PATCH", &format!("repos/{slug}"), Some(&body))
            });
        }
    }
    match &wanted.homepage {
        None => report.skip("homepage", "no manifest names one, and there is no site".to_string()),
        Some(homepage) if had("homepage").is_some_and(|h| same_address(&h, homepage)) => {
            report.holds("homepage", homepage.clone());
        }
        Some(homepage) => {
            let body = json!({ "homepage": homepage });
            let was = had("homepage").map(|h| format!(", was {h}")).unwrap_or_default();
            report.write("homepage", format!("{homepage}{was}"), || {
                api.send("PATCH", &format!("repos/{slug}"), Some(&body))
            });
        }
    }
}

/// The topics, as the manifests' keywords.
fn settle_topics(api: &Api, slug: &str, wanted: &Wanted, current: &Value, report: &mut Report) {
    if wanted.topics.is_empty() {
        report.skip("topics", "no manifest has `keywords` to take them from".to_string());
        return;
    }
    let had: Vec<String> = current["topics"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|t| t.as_str().map(str::to_string))
        .collect();
    let mut sorted_had = had.clone();
    sorted_had.sort();
    let mut sorted_wanted = wanted.topics.clone();
    sorted_wanted.sort();
    let list = wanted.topics.join(", ");
    let from = wanted.topics_from.join(" and ");
    if sorted_had == sorted_wanted {
        report.holds("topics", format!("{list} - the keywords of {from}"));
        return;
    }
    let was = if had.is_empty() {
        String::new()
    } else {
        format!("; were {}", had.join(", "))
    };
    let body = json!({ "names": wanted.topics });
    report.write("topics", format!("{list} - the keywords of {from}{was}"), || {
        api.send("PUT", &format!("repos/{slug}/topics"), Some(&body))
    });
}

/// GitHub Pages, built the way the repository builds them.
fn settle_pages(api: &Api, slug: &str, wanted: &Wanted, report: &mut Report) {
    let Some(domain) = &wanted.pages else {
        report.skip("pages", "no workflow deploys pages".to_string());
        return;
    };
    let at = domain.as_ref().map(|d| format!(", at {d}")).unwrap_or_default();
    let current = match api.find(&format!("repos/{slug}/pages")) {
        Ok(current) => current,
        Err(refusal) => return report.fail("pages", &refusal),
    };
    let Some(current) = current else {
        // Created first and then pointed at the domain: the call that
        // creates them takes how they are built, not where they are served.
        let body = json!({ "build_type": "workflow" });
        let cname = domain.clone();
        report.write("pages", format!("built by the workflow that deploys them{at}"), || {
            api.send("POST", &format!("repos/{slug}/pages"), Some(&body))?;
            match cname {
                Some(cname) => api.send("PUT", &format!("repos/{slug}/pages"), Some(&json!({ "cname": cname }))),
                None => Ok(Value::Null),
            }
        });
        return;
    };
    match pages_change(domain.as_deref(), &current) {
        None => report.holds("pages", format!("built by the workflow that deploys them{at}")),
        Some((body, was)) => report.write("pages", format!("built by the workflow that deploys them{at}; {was}"), || {
            api.send("PUT", &format!("repos/{slug}/pages"), Some(&body))
        }),
    }
}

/// What to change about pages that exist, and what they were; `None` when
/// they are built by the workflow and served where the repository says.
pub fn pages_change(domain: Option<&str>, current: &Value) -> Option<(Value, String)> {
    let build = current["build_type"].as_str().unwrap_or("legacy");
    let cname = current["cname"].as_str().filter(|c| !c.is_empty());
    let mut body = json!({ "build_type": "workflow" });
    let mut was = Vec::new();
    if build != "workflow" {
        was.push("were built from a branch".to_string());
    }
    if cname != domain {
        body["cname"] = domain.map_or(Value::Null, |d| json!(d));
        was.push(match cname {
            Some(c) => format!("were served at {c}"),
            None => "had no domain".to_string(),
        });
    }
    (!was.is_empty()).then(|| (body, was.join(" and ")))
}

/// Private vulnerability reporting: where SECURITY.md sends a reporter.
fn settle_reports(api: &Api, slug: &str, public: bool, report: &mut Report) {
    if !public {
        report.skip("reports", "a private repository has no private reporting to open".to_string());
        return;
    }
    let path = format!("repos/{slug}/private-vulnerability-reporting");
    match api.get(&path) {
        Ok(state) if state["enabled"].as_bool() == Some(true) => {
            report.holds("reports", "a vulnerability is reported in private, where SECURITY.md sends it".to_string());
        }
        Ok(_) => report.write(
            "reports",
            "a vulnerability is reported in private, where SECURITY.md sends it; was closed".to_string(),
            || api.send("PUT", &path, None),
        ),
        Err(refusal) => report.fail("reports", &refusal),
    }
}

/// The rule on the version tags: created only on a commit CI passed, never
/// moved or deleted. `checks` are what the CI's jobs report as.
pub fn tag_rules(checks: &[String]) -> Value {
    let mut rules = vec![json!({ "type": "deletion" }), json!({ "type": "update" })];
    if !checks.is_empty() {
        let required: Vec<Value> = checks.iter().map(|c| json!({ "context": c })).collect();
        rules.push(json!({
            "type": "required_status_checks",
            "parameters": {
                "strict_required_status_checks_policy": false,
                "do_not_enforce_on_create": false,
                "required_status_checks": required,
            }
        }));
    }
    json!({
        "name": TAG_RULES,
        "target": "tag",
        "enforcement": "active",
        "bypass_actors": [],
        "conditions": { "ref_name": { "include": [TAGS], "exclude": [] } },
        "rules": rules,
    })
}

/// The rule on the default branch: its history is never rewritten, and it
/// is never deleted.
pub fn branch_rules() -> Value {
    json!({
        "name": BRANCH_RULES,
        "target": "branch",
        "enforcement": "active",
        "bypass_actors": [],
        "conditions": { "ref_name": { "include": ["~DEFAULT_BRANCH"], "exclude": [] } },
        "rules": [{ "type": "deletion" }, { "type": "non_fast_forward" }],
    })
}

/// A ruleset as far as this command cares: what it applies to and what it
/// asks, in an order that does not depend on how GitHub lists it.
pub fn shape(ruleset: &Value) -> Value {
    let sorted = |value: &Value| {
        let mut items: Vec<String> = value.as_array().into_iter().flatten().filter_map(|v| v.as_str().map(str::to_string)).collect();
        items.sort();
        items
    };
    let mut rules: Vec<Value> = ruleset["rules"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|rule| {
            let kind = rule["type"].as_str().unwrap_or_default();
            if kind == "required_status_checks" {
                let parameters = &rule["parameters"];
                let mut checks: Vec<String> = parameters["required_status_checks"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|c| c["context"].as_str().map(str::to_string))
                    .collect();
                checks.sort();
                json!({
                    "type": kind,
                    "strict": parameters["strict_required_status_checks_policy"].as_bool().unwrap_or(false),
                    "on_create": !parameters["do_not_enforce_on_create"].as_bool().unwrap_or(false),
                    "checks": checks,
                })
            } else {
                json!({ "type": kind })
            }
        })
        .collect();
    rules.sort_by_key(|r| r["type"].as_str().unwrap_or_default().to_string());
    json!({
        "target": ruleset["target"],
        "enforcement": ruleset["enforcement"],
        "include": sorted(&ruleset["conditions"]["ref_name"]["include"]),
        "exclude": sorted(&ruleset["conditions"]["ref_name"]["exclude"]),
        "bypass": ruleset["bypass_actors"].as_array().map_or(0, Vec::len),
        "rules": rules,
    })
}

/// The jobs of the newest green run of the release's workflow on the
/// default branch: the checks a commit has passed when CI passed it.
fn checks_of(api: &Api, slug: &str, workflow: &str, branch: &str) -> Result<Option<Vec<String>>, Refusal> {
    let runs = api.find(&format!(
        "repos/{slug}/actions/workflows/{workflow}/runs?branch={branch}&status=success&per_page=1"
    ))?;
    let Some(run) = runs.as_ref().and_then(|r| r["workflow_runs"].as_array()).and_then(|r| r.first()) else {
        return Ok(None);
    };
    let id = run["id"].as_u64().unwrap_or_default();
    let jobs = api.get(&format!("repos/{slug}/actions/runs/{id}/jobs?per_page=100"))?;
    let mut names: Vec<String> = jobs["jobs"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|j| j["name"].as_str().map(str::to_string))
        .collect();
    names.sort();
    names.dedup();
    Ok(Some(names))
}

/// The two rules: on the version tags, and on the default branch.
fn settle_rules(api: &Api, slug: &str, wanted: &Wanted, branch: &str, report: &mut Report) {
    let listed = match api.find(&format!("repos/{slug}/rulesets?includes_parents=false")) {
        Ok(listed) => listed.unwrap_or(Value::Array(Vec::new())),
        Err(refusal) => {
            report.fail("tags", &refusal);
            report.fail(branch, &refusal);
            return;
        }
    };
    let find = |name: &str| -> Result<Option<Value>, Refusal> {
        let Some(id) = listed
            .as_array()
            .into_iter()
            .flatten()
            .find(|r| r["name"].as_str() == Some(name))
            .and_then(|r| r["id"].as_u64())
        else {
            return Ok(None);
        };
        api.get(&format!("repos/{slug}/rulesets/{id}")).map(Some)
    };

    // The checks a tag waits for are the CI's jobs as GitHub names them,
    // read off a run that passed rather than out of the workflow: a matrix
    // or a job's `name:` is spelt the way GitHub reports it only there.
    let checks = match &wanted.ci {
        Some(workflow) => match checks_of(api, slug, workflow, branch) {
            Ok(checks) => checks,
            Err(refusal) => {
                report.fail("tags", &refusal);
                return;
            }
        },
        None => None,
    };
    let required = checks.clone().unwrap_or_default();
    let tags_text = match &checks {
        Some(checks) if !checks.is_empty() => format!(
            "v* are created only on a commit {} passed - {}; never moved or deleted",
            wanted.ci.as_deref().unwrap_or("CI"),
            some_of(checks)
        ),
        _ => "v* are never moved or deleted".to_string(),
    };
    let unchecked = match (&wanted.ci, &checks) {
        (None, _) => Some("no CI workflow here to hold a release to: release.toml names none, and there is no ci.yml".to_string()),
        (Some(workflow), None) => Some(format!(
            "{workflow} has not passed on {branch} yet, so there are no checks to require of a tag: `lyrn repo` again after it has"
        )),
        _ => None,
    };
    settle_ruleset(api, slug, "tags", &tag_rules(&required), find(TAG_RULES), &tags_text, report);
    if let Some(unchecked) = unchecked {
        report.hand("tags", "a tag is not held to CI yet".to_string(), unchecked);
    }
    settle_ruleset(api, slug, branch, &branch_rules(), find(BRANCH_RULES), "never force-pushed or deleted", report);
}

/// The checks named, a few of them when there are many: a matrix of seventy
/// jobs is one line of the report, not seventy names.
pub fn some_of(checks: &[String]) -> String {
    const NAMED: usize = 4;
    match checks.len() {
        1 => format!("its check {}", checks[0]),
        n if n <= NAMED => format!("its {n} checks: {}", checks.join(", ")),
        n => format!("its {n} checks: {} and {} more", checks[..NAMED].join(", "), n - NAMED),
    }
}

fn settle_ruleset(api: &Api, slug: &str, name: &str, wanted: &Value, current: Result<Option<Value>, Refusal>, text: &str, report: &mut Report) {
    match current {
        Err(refusal) => report.fail(name, &refusal),
        Ok(Some(current)) if shape(&current) == shape(wanted) => report.holds(name, text.to_string()),
        Ok(Some(current)) => {
            let id = current["id"].as_u64().unwrap_or_default();
            report.write(name, format!("{text}; the rule GitHub kept asked otherwise"), || {
                api.send("PUT", &format!("repos/{slug}/rulesets/{id}"), Some(wanted))
            });
        }
        Ok(None) => report.write(name, text.to_string(), || api.send("POST", &format!("repos/{slug}/rulesets"), Some(wanted))),
    }
}

/// The social preview: GitHub has no API to upload one, so the command says
/// whether the picture GitHub shows is the one `lyrn assets` drew.
fn settle_preview(api: &Api, slug: &str, root: &Path, report: &mut Report) {
    let Ok(drawn) = std::fs::read(root.join(SOCIAL)) else {
        report.skip("social preview", format!("no {SOCIAL}: `lyrn assets` draws it from the banner"));
        return;
    };
    let settings = format!("upload {SOCIAL} at https://github.com/{slug}/settings → Social preview: GitHub has no API for it");
    let (owner, name) = slug.split_once('/').unwrap_or((slug, ""));
    let query = format!(r#"query {{ repository(owner: "{owner}", name: "{name}") {{ usesCustomOpenGraphImage openGraphImageUrl }} }}"#);
    let shown = match api.graphql(&query) {
        Ok(data) => data["repository"].clone(),
        Err(refusal) => return report.fail("social preview", &refusal),
    };
    if shown["usesCustomOpenGraphImage"].as_bool() != Some(true) {
        report.hand("social preview", "GitHub shows a picture of its own".to_string(), settings);
        return;
    }
    let Some(url) = shown["openGraphImageUrl"].as_str() else {
        report.hand("social preview", "GitHub does not say where its picture is".to_string(), settings);
        return;
    };
    match api.download(url) {
        Ok(bytes) if same_picture(&bytes, &drawn) => report.holds("social preview", format!("{SOCIAL}, as drawn")),
        Ok(_) => report.hand("social preview", format!("GitHub shows another picture than {SOCIAL}"), settings),
        Err(refusal) => report.fail("social preview", &refusal),
    }
}

/// Whether two PNGs are one picture, pixel for pixel: GitHub may store the
/// upload in other bytes.
pub fn same_picture(a: &[u8], b: &[u8]) -> bool {
    if a == b {
        return true;
    }
    if dimensions(a).is_none() || dimensions(a) != dimensions(b) {
        return false;
    }
    match (crate::mark::ico::pixels(a), crate::mark::ico::pixels(b)) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    }
}

/// A PNG's width and height, from its header.
fn dimensions(png: &[u8]) -> Option<(u32, u32)> {
    if !png.starts_with(b"\x89PNG\r\n\x1a\n") || png.get(12..16)? != b"IHDR" {
        return None;
    }
    let number = |at: usize| png.get(at..at + 4).map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]));
    Some((number(16)?, number(20)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_keyword_is_made_a_topic() {
        assert_eq!(topic("cli").as_deref(), Some("cli"));
        assert_eq!(topic("Command Line").as_deref(), Some("command-line"));
        assert_eq!(topic("command_line_utilities").as_deref(), Some("command-line-utilities"));
        assert_eq!(topic("  --rust--  ").as_deref(), Some("rust"));
        assert_eq!(topic("c++"), Some("c".to_string()));
        assert_eq!(topic("!!!"), None);
        assert_eq!(topic(&"a".repeat(51)), None);
    }

    #[test]
    fn a_trailing_slash_is_the_same_address() {
        assert!(same_address("https://lyrn.lacodda.com", "https://lyrn.lacodda.com/"));
        assert!(!same_address("https://lyrn.lacodda.com", "https://lacodda.github.io/lyrn/"));
    }

    /// Pages built from a branch are moved to the workflow, keeping the
    /// domain; pages already so are left alone.
    #[test]
    fn pages_are_moved_to_the_workflow_that_deploys_them() {
        let legacy = json!({ "build_type": "legacy", "cname": "lyrn.lacodda.com" });
        let (body, was) = pages_change(Some("lyrn.lacodda.com"), &legacy).unwrap();
        assert_eq!(body, json!({ "build_type": "workflow" }));
        assert_eq!(was, "were built from a branch");

        let built = json!({ "build_type": "workflow", "cname": "lyrn.lacodda.com" });
        assert_eq!(pages_change(Some("lyrn.lacodda.com"), &built), None);

        let (body, was) = pages_change(None, &built).unwrap();
        assert_eq!(body, json!({ "build_type": "workflow", "cname": null }));
        assert_eq!(was, "were served at lyrn.lacodda.com");

        let bare = json!({ "build_type": "workflow", "cname": null });
        let (body, _) = pages_change(Some("demo.dev"), &bare).unwrap();
        assert_eq!(body["cname"], "demo.dev");
    }

    /// A tag is held to every check the CI's jobs report, and to nothing when
    /// there are none to name; a rule GitHub lists in another order is the
    /// same rule.
    #[test]
    fn the_tag_rule_requires_the_ci_s_checks() {
        let checks = vec!["ubuntu-latest".to_string(), "msrv".to_string()];
        let wanted = tag_rules(&checks);
        assert_eq!(wanted["conditions"]["ref_name"]["include"], json!(["refs/tags/v*"]));
        let kinds: Vec<&str> = wanted["rules"].as_array().unwrap().iter().map(|r| r["type"].as_str().unwrap()).collect();
        assert_eq!(kinds, ["deletion", "update", "required_status_checks"]);
        let required = &wanted["rules"][2]["parameters"];
        assert_eq!(required["do_not_enforce_on_create"], false, "a tag is created only on a commit that passed");
        assert_eq!(
            required["required_status_checks"],
            json!([{ "context": "ubuntu-latest" }, { "context": "msrv" }])
        );

        let unchecked = tag_rules(&[]);
        assert_eq!(unchecked["rules"].as_array().unwrap().len(), 2);

        // As GitHub returns it: other fields, another order.
        let listed = json!({
            "id": 7, "name": "Release tags", "target": "tag", "enforcement": "active", "bypass_actors": [],
            "source_type": "Repository",
            "conditions": { "ref_name": { "exclude": [], "include": ["refs/tags/v*"] } },
            "rules": [
                { "type": "required_status_checks", "parameters": {
                    "strict_required_status_checks_policy": false, "do_not_enforce_on_create": false,
                    "required_status_checks": [{ "context": "msrv" }, { "context": "ubuntu-latest" }] } },
                { "type": "update" }, { "type": "deletion" }
            ]
        });
        assert_eq!(shape(&listed), shape(&wanted));
        assert_ne!(shape(&listed), shape(&tag_rules(&["msrv".to_string()])), "a check gone is a change");
        assert_ne!(shape(&listed), shape(&unchecked));
        let mut evaded = listed.clone();
        evaded["bypass_actors"] = json!([{ "actor_id": 5, "actor_type": "RepositoryRole", "bypass_mode": "always" }]);
        assert_ne!(shape(&evaded), shape(&wanted), "a rule someone may step around is not the rule");
        let mut disabled = listed;
        disabled["enforcement"] = json!("disabled");
        assert_ne!(shape(&disabled), shape(&wanted));
    }

    /// The default branch is never rewritten, and nothing is asked of a
    /// push to it: a branch that waits for checks refuses every push.
    #[test]
    fn the_branch_rule_keeps_history_and_lets_pushes_in() {
        let rules = branch_rules();
        let kinds: Vec<&str> = rules["rules"].as_array().unwrap().iter().map(|r| r["type"].as_str().unwrap()).collect();
        assert_eq!(kinds, ["deletion", "non_fast_forward"]);
        assert_eq!(rules["conditions"]["ref_name"]["include"], json!(["~DEFAULT_BRANCH"]));
    }

    #[test]
    fn many_checks_are_named_by_a_few() {
        let names = |n: usize| (1..=n).map(|i| format!("job{i}")).collect::<Vec<_>>();
        assert_eq!(some_of(&names(1)), "its check job1");
        assert_eq!(some_of(&names(3)), "its 3 checks: job1, job2, job3");
        assert_eq!(some_of(&names(70)), "its 70 checks: job1, job2, job3, job4 and 66 more");
    }

    #[test]
    fn a_picture_is_its_pixels() {
        let red = crate::mark::ico::tests::png_of(4, [255, 0, 0, 255]);
        let blue = crate::mark::ico::tests::png_of(4, [0, 0, 255, 255]);
        assert!(same_picture(&red, &red));
        assert!(!same_picture(&red, &blue));
        assert!(!same_picture(&red, b"not a picture"));
        let larger = crate::mark::ico::tests::png_of(8, [255, 0, 0, 255]);
        assert!(!same_picture(&red, &larger));
    }
}
