//! GitHub workflows, read as the structure GitHub reads them.
//!
//! Several rules ask what a job runs and where: which actions a step uses,
//! which directory a script starts in, what a job is called. Lines matched by
//! pattern would get the directory wrong as soon as it is set once for the
//! whole job under `defaults`, so the files are parsed.

use std::path::Path;

use yaml_rust2::{Yaml, YamlLoader};

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
}

/// A step of a job.
pub struct Step<'a> {
    pub uses: Option<&'a str>,
    pub run: Option<&'a str>,
    pub working_directory: Option<&'a str>,
    with: &'a Yaml,
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
}

/// Every workflow under `.github/workflows/`, and the ones that do not parse.
pub fn load(root: &Path, files: &[String]) -> (Vec<Workflow>, Vec<(String, String)>) {
    let mut loaded = Vec::new();
    let mut broken = Vec::new();
    for file in files {
        let Some(name) = file.strip_prefix(".github/workflows/") else { continue };
        if name.contains('/') || !(name.ends_with(".yml") || name.ends_with(".yaml")) {
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
                })
            })
            .collect()
    }

    /// The file's own name, for findings: `ci.yml`.
    pub fn name(&self) -> &str {
        self.file.rsplit('/').next().unwrap_or(&self.file)
    }
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
}
