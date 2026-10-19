//! GitHub workflows, read as the structure GitHub reads them.
//!
//! Several commands ask what a job runs and where: which actions a step uses,
//! which directory a script starts in, what a job is called, which
//! combinations its matrix makes and whether a push starts it at all. Lines
//! matched by pattern would get the directory wrong as soon as it is set once
//! for the whole job under `defaults`, so the files are parsed - and a matrix
//! is expanded by GitHub's rules, include and exclude and all, because a
//! combination read almost right is a job CI runs and nobody else does.

pub mod expr;

use std::path::Path;

use regex::Regex;
use yaml_rust2::{Yaml, YamlLoader};

use self::expr::Value;

/// One workflow file.
pub struct Workflow {
    /// Its path in the repository, as `.github/workflows/ci.yml`.
    pub file: String,
    /// The text as written: comments carry what a pinned commit stands for.
    pub text: String,
    doc: Yaml,
}

/// A job of a workflow.
pub struct Job<'a> {
    pub id: &'a str,
    pub name: Option<&'a str>,
    /// A reusable workflow the job calls instead of running steps.
    pub uses: Option<&'a str>,
    pub steps: Vec<Step<'a>>,
    /// `defaults.run.working-directory`, the job's or else the workflow's.
    pub working_directory: Option<&'a str>,
    /// `defaults.run.shell`, the job's or else the workflow's.
    pub shell: Option<&'a str>,
    /// The job as written, for what only running it needs.
    pub doc: &'a Yaml,
}

/// A step of a job.
pub struct Step<'a> {
    pub uses: Option<&'a str>,
    pub run: Option<&'a str>,
    pub working_directory: Option<&'a str>,
    with: &'a Yaml,
    /// The step as written, for what only running it needs.
    pub doc: &'a Yaml,
}

impl Step<'_> {
    /// An input given under `with:`, as text whatever YAML made of it:
    /// `node-version: 22` is a number to the parser and a version to GitHub.
    pub fn input(&self, key: &str) -> Option<String> {
        scalar(&self.with[key])
    }

    /// The directory a `run:` script starts in.
    pub fn run_directory<'b>(&'b self, job: &'b Job) -> Option<&'b str> {
        self.working_directory.or(job.working_directory)
    }

    /// What the step is called in a run's log: its name, else what it runs.
    pub fn title(&self) -> String {
        if let Some(name) = self.doc["name"].as_str() {
            return name.to_string();
        }
        if let Some(uses) = self.uses {
            return uses.to_string();
        }
        let first = self.run.and_then(|r| r.lines().map(str::trim).find(|l| !l.is_empty())).unwrap_or_default();
        format!("Run {first}")
    }
}

/// Every workflow under `.github/workflows/`, and the ones that do not parse.
pub fn load(root: &Path, files: &[String]) -> (Vec<Workflow>, Vec<(String, String)>) {
    let mut loaded = Vec::new();
    let mut broken = Vec::new();
    for file in files {
        if !is_workflow(file) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(root.join(file)) else { continue };
        match parse(file, text) {
            Ok(workflow) => loaded.push(workflow),
            Err(reason) => broken.push((file.clone(), reason)),
        }
    }
    (loaded, broken)
}

/// Whether a repository path is a workflow GitHub would run.
pub fn is_workflow(file: &str) -> bool {
    file.strip_prefix(".github/workflows/")
        .is_some_and(|name| !name.contains('/') && (name.ends_with(".yml") || name.ends_with(".yaml")))
}

pub fn parse(file: &str, text: String) -> Result<Workflow, String> {
    let mut docs = YamlLoader::load_from_str(&text).map_err(|e| e.to_string())?;
    if docs.is_empty() {
        return Err("the file is empty".to_string());
    }
    Ok(Workflow {
        file: file.to_string(),
        text,
        doc: docs.swap_remove(0),
    })
}

impl Workflow {
    pub fn jobs(&self) -> Vec<Job<'_>> {
        let workflow_directory = self.doc["defaults"]["run"]["working-directory"].as_str();
        let workflow_shell = self.doc["defaults"]["run"]["shell"].as_str();
        let Some(jobs) = self.doc["jobs"].as_hash() else { return Vec::new() };
        jobs.iter()
            .filter_map(|(id, job)| {
                let id = id.as_str()?;
                let steps = job["steps"]
                    .as_vec()
                    .map(|steps| {
                        steps
                            .iter()
                            .map(|step| Step {
                                uses: step["uses"].as_str(),
                                run: step["run"].as_str(),
                                working_directory: step["working-directory"].as_str(),
                                with: &step["with"],
                                doc: step,
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                Some(Job {
                    id,
                    name: job["name"].as_str(),
                    uses: job["uses"].as_str(),
                    steps,
                    working_directory: job["defaults"]["run"]["working-directory"].as_str().or(workflow_directory),
                    shell: job["defaults"]["run"]["shell"].as_str().or(workflow_shell),
                    doc: job,
                })
            })
            .collect()
    }

    /// The file's own name, for findings: `ci.yml`.
    pub fn name(&self) -> &str {
        self.file.rsplit('/').next().unwrap_or(&self.file)
    }

    /// The workflow as written.
    pub fn doc(&self) -> &Yaml {
        &self.doc
    }

    /// Whether pushing `branch` starts the workflow, given the files the push
    /// changes - `None` when they are not known, which no path filter can
    /// then rule out.
    pub fn starts_on_push(&self, branch: &str, changed: Option<&[String]>) -> Start {
        // `on` is a word to YAML 1.2 and a boolean to 1.1; a file written for
        // either is read.
        let on = match &self.doc["on"] {
            Yaml::BadValue => self.doc.as_hash().and_then(|h| h.get(&Yaml::Boolean(true))).unwrap_or(&Yaml::BadValue),
            on => on,
        };
        let push = match on {
            Yaml::String(event) if event == "push" => &Yaml::Null,
            Yaml::Array(events) if events.iter().any(|e| e.as_str() == Some("push")) => &Yaml::Null,
            Yaml::Hash(events) => match events.get(&Yaml::String("push".to_string())) {
                Some(push) => push,
                None => return Start::Not(format!("starts on {}, not on a push", event_names(on))),
            },
            _ => return Start::Not(format!("starts on {}, not on a push", event_names(on))),
        };
        let list = |key: &str| -> Vec<String> {
            match &push[key] {
                Yaml::Array(items) => items.iter().filter_map(scalar).collect(),
                other => scalar(other).into_iter().collect(),
            }
        };
        let has = |key: &str| !matches!(push[key], Yaml::BadValue | Yaml::Null);
        if (has("tags") || has("tags-ignore")) && !has("branches") && !has("branches-ignore") {
            return Start::Not("starts on a pushed tag, not a branch".to_string());
        }
        if has("branches") && !filtered_in(&list("branches"), &[branch.to_string()]) {
            return Start::Not(format!("starts on pushes to {}, not to `{branch}`", list("branches").join(", ")));
        }
        if has("branches-ignore") && list("branches-ignore").iter().any(|p| pattern_matches(p, branch)) {
            return Start::Not(format!("leaves pushes to `{branch}` out"));
        }
        let Some(changed) = changed else { return Start::Yes };
        if has("paths") && !filtered_in(&list("paths"), changed) {
            return Start::Not(format!("starts when {} change, and this push changes none of them", list("paths").join(", ")));
        }
        if has("paths-ignore") && !changed.is_empty() && changed.iter().all(|file| list("paths-ignore").iter().any(|p| pattern_matches(p, file))) {
            return Start::Not(format!("leaves out pushes that change only {}", list("paths-ignore").join(", ")));
        }
        Start::Yes
    }
}

/// Whether a push starts a workflow, and why not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Start {
    Yes,
    Not(String),
}

fn event_names(on: &Yaml) -> String {
    let names: Vec<String> = match on {
        Yaml::String(event) => vec![event.clone()],
        Yaml::Array(events) => events.iter().filter_map(scalar).collect(),
        Yaml::Hash(events) => events.keys().filter_map(scalar).collect(),
        _ => Vec::new(),
    };
    if names.is_empty() { "nothing".to_string() } else { names.join(", ") }
}

/// Whether any of `items` passes a GitHub filter: the patterns in order, a
/// `!` taking back what the ones before it matched.
fn filtered_in(patterns: &[String], items: &[String]) -> bool {
    items.iter().any(|item| {
        let mut matched = false;
        for pattern in patterns {
            match pattern.strip_prefix('!') {
                Some(negated) if pattern_matches(negated, item) => matched = false,
                None if pattern_matches(pattern, item) => matched = true,
                _ => {}
            }
        }
        matched
    })
}

/// A branch or path filter of GitHub's: anchored at both ends, `*` stays
/// within a directory, `**` crosses them, and `?` and `+` repeat the
/// character before them as in a regular expression.
pub fn pattern_matches(pattern: &str, text: &str) -> bool {
    let mut regex = String::from("^");
    let chars: Vec<char> = pattern.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '*' if chars.get(i + 1) == Some(&'*') => {
                regex.push_str(".*");
                i += 2;
                continue;
            }
            '*' => regex.push_str("[^/]*"),
            '?' | '+' if i > 0 => regex.push(chars[i]),
            '[' => match chars[i..].iter().position(|c| *c == ']') {
                Some(end) => {
                    regex.extend(&chars[i..=i + end]);
                    i += end + 1;
                    continue;
                }
                None => regex.push_str("\\["),
            },
            c => regex.push_str(&regex::escape(&c.to_string())),
        }
        i += 1;
    }
    regex.push('$');
    Regex::new(&regex).is_ok_and(|r| r.is_match(text))
}

/// One combination of a job's matrix, its keys in the order written.
pub type Combination = Vec<(String, Yaml)>;

/// The combinations a job runs as, by GitHub's rules: the product of the
/// matrix's lists, less what `exclude` names, with each `include` merged into
/// every original combination it does not contradict - or added as one of
/// its own when it fits none. A job without a matrix runs once.
pub fn combinations(job: &Yaml) -> Result<Vec<Combination>, String> {
    let matrix = match &job["strategy"]["matrix"] {
        Yaml::BadValue | Yaml::Null => return Ok(vec![Vec::new()]),
        Yaml::Hash(matrix) => matrix,
        Yaml::String(text) if text.contains("${{") => return Err("its matrix is computed when the workflow runs".to_string()),
        _ => return Err("its matrix is not a mapping".to_string()),
    };
    let mut base: Vec<Combination> = vec![Vec::new()];
    let mut keys: Vec<String> = Vec::new();
    for (key, values) in matrix {
        let Some(key) = scalar(key) else { continue };
        if key == "include" || key == "exclude" {
            continue;
        }
        let values = match values {
            Yaml::Array(values) => values.clone(),
            Yaml::String(text) if text.contains("${{") => return Err(format!("`matrix.{key}` is computed when the workflow runs")),
            other => vec![other.clone()],
        };
        keys.push(key.clone());
        base = base
            .into_iter()
            .flat_map(|combination| {
                let key = key.clone();
                values.iter().map(move |value| {
                    let mut next = combination.clone();
                    next.push((key.clone(), value.clone()));
                    next
                })
            })
            .collect();
    }
    let entries = |name: &str| -> Vec<Combination> {
        matrix
            .get(&Yaml::String(name.to_string()))
            .and_then(Yaml::as_vec)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Yaml::as_hash)
                    .map(|entry| entry.iter().filter_map(|(k, v)| Some((scalar(k)?, v.clone()))).collect())
                    .collect()
            })
            .unwrap_or_default()
    };
    if keys.is_empty() {
        let included = entries("include");
        return Ok(if included.is_empty() { vec![Vec::new()] } else { included });
    }
    for exclude in entries("exclude") {
        base.retain(|combination| !exclude.iter().all(|(k, v)| lookup(combination, k).is_some_and(|c| same(c, v))));
    }
    let mut added: Vec<Combination> = Vec::new();
    for include in entries("include") {
        let mut merged = false;
        for combination in &mut base {
            let fits = include
                .iter()
                .all(|(k, v)| !keys.contains(k) || lookup(combination, k).is_some_and(|c| same(c, v)));
            if fits {
                merged = true;
                for (k, v) in &include {
                    match combination.iter_mut().find(|(ck, _)| ck == k) {
                        Some((_, existing)) => *existing = v.clone(),
                        None => combination.push((k.clone(), v.clone())),
                    }
                }
            }
        }
        if !merged {
            added.push(include);
        }
    }
    base.extend(added);
    Ok(base)
}

fn lookup<'a>(combination: &'a Combination, key: &str) -> Option<&'a Yaml> {
    combination.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

fn same(a: &Yaml, b: &Yaml) -> bool {
    match (scalar(a), scalar(b)) {
        (Some(a), Some(b)) => a == b,
        _ => a == b,
    }
}

/// A combination as the `matrix` context.
pub fn matrix_value(combination: &Combination) -> Value {
    Value::Object(combination.iter().map(|(k, v)| (k.clone(), Value::from_yaml(v))).collect())
}

/// What a job is called in a run's log: its name with the combination's
/// expressions filled in, else its id with the combination's values.
pub fn job_title(job: &Job, combination: &Combination) -> String {
    if let Some(name) = job.name {
        let scope = expr::Scope::new().with("matrix", matrix_value(combination));
        if let Ok(title) = expr::interpolate(name, &scope) {
            return title;
        }
    }
    if combination.is_empty() {
        return job.id.to_string();
    }
    let values: Vec<String> = combination
        .iter()
        .map(|(_, v)| match v {
            Yaml::Hash(_) | Yaml::Array(_) => "…".to_string(),
            v => scalar(v).unwrap_or_default(),
        })
        .collect();
    format!("{} ({})", job.id, values.join(", "))
}

/// A scalar as text, whichever kind the parser read it as.
pub fn scalar(value: &Yaml) -> Option<String> {
    match value {
        Yaml::String(s) | Yaml::Real(s) => Some(s.clone()),
        Yaml::Integer(i) => Some(i.to_string()),
        Yaml::Boolean(b) => Some(b.to_string()),
        _ => None,
    }
}

/// A directory as a repository path: `./docs/` and `docs` are the same place,
/// and `.` is the top. `None` for one GitHub only knows at run time.
pub fn normalize_dir(dir: &str) -> Option<String> {
    let dir = dir.trim().trim_matches(|c| c == '"' || c == '\'');
    if dir.contains("${{") || dir.contains('$') || dir.starts_with('/') || dir.contains(':') {
        return None;
    }
    let mut parts: Vec<&str> = Vec::new();
    for part in dir.split(['/', '\\']) {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            part => parts.push(part),
        }
    }
    Some(if parts.is_empty() { ".".to_string() } else { parts.join("/") })
}

/// `inner` taken from `base`, both repository paths.
pub fn join_dir(base: &str, inner: &str) -> Option<String> {
    if base == "." {
        normalize_dir(inner)
    } else {
        normalize_dir(&format!("{base}/{inner}"))
    }
}

/// The directories a job's directory stands for once its matrix is known:
/// `${{ matrix.project }}` is each project the matrix lists. `None` when the
/// directory needs something only a run knows.
pub fn directories(dir: &str, job: &Job) -> Option<Vec<String>> {
    if !dir.contains("${{") {
        return normalize_dir(dir).map(|d| vec![d]);
    }
    let combinations = combinations(job.doc).ok()?;
    let mut dirs = Vec::new();
    for combination in &combinations {
        let scope = expr::Scope::new().with("matrix", matrix_value(combination));
        let dir = normalize_dir(&expr::interpolate(dir, &scope).ok()?)?;
        if !dirs.contains(&dir) {
            dirs.push(dir);
        }
    }
    Some(dirs)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CI: &str = "\
name: CI
on: [push]
defaults:
  run:
    working-directory: app
jobs:
  site:
    name: Site
    defaults:
      run:
        working-directory: docs
    steps:
      - uses: actions/checkout@v7
      - run: pnpm audit
      - run: cargo test
        working-directory: crates/core
      - uses: actions/setup-node@v7
        with:
          node-version: 22
  rest:
    steps:
      - run: echo hi
  shared:
    uses: owner/repo/.github/workflows/release.yml@v1
";

    #[test]
    fn a_step_starts_where_the_step_or_its_job_says() {
        let workflow = parse(".github/workflows/ci.yml", CI.to_string()).unwrap();
        let jobs = workflow.jobs();
        assert_eq!(jobs.len(), 3);
        let site = &jobs[0];
        assert_eq!((site.id, site.name), ("site", Some("Site")));
        assert_eq!(site.steps[1].run_directory(site), Some("docs"));
        assert_eq!(site.steps[2].run_directory(site), Some("crates/core"));
        assert_eq!(site.steps[3].input("node-version").as_deref(), Some("22"));
        // The workflow's default applies where the job sets none.
        assert_eq!(jobs[1].steps[0].run_directory(&jobs[1]), Some("app"));
        assert_eq!(jobs[2].uses, Some("owner/repo/.github/workflows/release.yml@v1"));
    }

    #[test]
    fn a_file_github_would_refuse_is_reported() {
        assert!(parse(".github/workflows/ci.yml", "jobs:\n  a: [unclosed\n".to_string()).is_err());
    }

    #[test]
    fn directories_are_compared_as_places() {
        assert_eq!(normalize_dir("./docs/").as_deref(), Some("docs"));
        assert_eq!(normalize_dir(".").as_deref(), Some("."));
        assert_eq!(normalize_dir("a/../b").as_deref(), Some("b"));
        assert_eq!(normalize_dir("${{ runner.temp }}/demo"), None);
        assert_eq!(normalize_dir("$RUNNER_TEMP/demo"), None);
        assert_eq!(join_dir("docs", "../site").as_deref(), Some("site"));
        assert_eq!(join_dir(".", "frontend").as_deref(), Some("frontend"));
    }

    fn matrix(yaml: &str) -> Vec<String> {
        let doc = &YamlLoader::load_from_str(yaml).unwrap()[0];
        combinations(doc)
            .unwrap()
            .iter()
            .map(|c| {
                c.iter()
                    .map(|(k, v)| format!("{k}={}", scalar(v).unwrap_or_default()))
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .collect()
    }

    #[test]
    fn a_job_without_a_matrix_runs_once() {
        assert_eq!(matrix("runs-on: ubuntu-latest\n"), vec![String::new()]);
    }

    #[test]
    fn a_matrix_is_the_product_of_its_lists_less_what_is_excluded() {
        let yaml = "strategy:\n  matrix:\n    os: [ubuntu, windows]\n    addons: ['', keyring]\n    exclude:\n      - { os: windows, addons: keyring }\n";
        assert_eq!(matrix(yaml), ["os=ubuntu,addons=", "os=ubuntu,addons=keyring", "os=windows,addons="]);
    }

    /// GitHub's own example, word for word: an include joins every original
    /// combination it does not contradict, may overwrite what an earlier
    /// include added, and stands alone when it fits none.
    #[test]
    fn include_follows_github_s_rules() {
        let yaml = "strategy:\n  matrix:\n    fruit: [apple, pear]\n    animal: [cat, dog]\n    include:\n      - color: green\n      - color: pink\n        animal: cat\n      - fruit: apple\n        shape: circle\n      - fruit: banana\n      - fruit: banana\n        animal: cat\n";
        assert_eq!(
            matrix(yaml),
            [
                "fruit=apple,animal=cat,color=pink,shape=circle",
                "fruit=apple,animal=dog,color=green,shape=circle",
                "fruit=pear,animal=cat,color=pink",
                "fruit=pear,animal=dog,color=green",
                "fruit=banana",
                "fruit=banana,animal=cat",
            ]
        );
    }

    #[test]
    fn a_matrix_of_includes_alone_is_its_includes() {
        let yaml = "strategy:\n  matrix:\n    include:\n      - { os: ubuntu-latest, addons: router }\n      - { os: ubuntu-latest, addons: auth }\n";
        assert_eq!(matrix(yaml), ["os=ubuntu-latest,addons=router", "os=ubuntu-latest,addons=auth"]);
    }

    #[test]
    fn a_matrix_known_only_at_run_time_is_said_so() {
        let doc = &YamlLoader::load_from_str("strategy:\n  matrix: ${{ fromJSON(needs.plan.outputs.matrix) }}\n").unwrap()[0];
        assert!(combinations(doc).is_err());
    }

    #[test]
    fn a_matrix_directory_is_every_directory_it_lists() {
        let yaml = "jobs:\n  audit:\n    strategy:\n      matrix:\n        project: [web, docs]\n    defaults:\n      run:\n        working-directory: ${{ matrix.project }}\n    steps:\n      - run: pnpm audit\n";
        let workflow = parse("audit.yml", yaml.to_string()).unwrap();
        let job = &workflow.jobs()[0];
        assert_eq!(
            directories(job.working_directory.unwrap(), job),
            Some(vec!["web".to_string(), "docs".to_string()])
        );
        assert_eq!(directories("${{ runner.temp }}/x", job), None);
    }

    fn starts(on: &str, branch: &str, changed: Option<&[&str]>) -> Start {
        let workflow = parse("w.yml", format!("{on}\njobs: {{}}\n")).unwrap();
        let changed: Option<Vec<String>> = changed.map(|c| c.iter().map(|s| s.to_string()).collect());
        workflow.starts_on_push(branch, changed.as_deref())
    }

    #[test]
    fn a_push_starts_what_its_filters_let_through() {
        assert_eq!(starts("on: push", "main", None), Start::Yes);
        assert_eq!(starts("on: [push, pull_request]", "main", None), Start::Yes);
        assert_eq!(starts("on:\n  push:\n    branches: [main]", "main", None), Start::Yes);
        assert!(matches!(starts("on:\n  push:\n    branches: [main]", "dev", None), Start::Not(_)));
        assert!(matches!(starts("on:\n  push:\n    tags: ['v*']", "main", None), Start::Not(_)));
        assert!(matches!(starts("on: [pull_request]", "main", None), Start::Not(_)));
        assert!(matches!(starts("on:\n  workflow_dispatch:", "main", None), Start::Not(_)));
        let docs_only: &[&str] = &["docs/src/index.md"];
        let code: &[&str] = &["docs/a.md", "src/main.rs"];
        let ci = "on:\n  push:\n    branches: [main]\n    paths-ignore: ['docs/**']";
        assert!(matches!(starts(ci, "main", Some(docs_only)), Start::Not(_)));
        assert_eq!(starts(ci, "main", Some(code)), Start::Yes);
        let docs = "on:\n  push:\n    paths: ['docs/**', '!docs/drafts/**']";
        assert_eq!(starts(docs, "main", Some(docs_only)), Start::Yes);
        assert!(matches!(starts(docs, "main", Some(&["docs/drafts/x.md"])), Start::Not(_)));
    }

    #[test]
    fn filter_patterns_are_github_s() {
        assert!(pattern_matches("docs/**", "docs/a/b.md"));
        assert!(pattern_matches("*.md", "README.md"));
        assert!(!pattern_matches("*.md", "docs/README.md"));
        assert!(pattern_matches("**.md", "docs/README.md"));
        assert!(pattern_matches("release/*", "release/1.0"));
        assert!(!pattern_matches("release/*", "release/1.0/x"));
        assert!(pattern_matches("v[12].*", "v1.4"));
    }
}
