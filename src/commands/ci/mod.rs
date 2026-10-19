//! `lyrn ci`: the CI a push of this commit would start, run here first.
//!
//! A red CI on the release commit is the most expensive kind: the tag waits,
//! the fix is one more commit on a history that was meant to be finished, and
//! the reason is usually something a terminal could have shown - a file left
//! untracked that the commit needs, a lint the newest toolchain added, a
//! promised `rust-version` the code has outgrown. So the run reads the
//! workflows the commit carries, decides which of them a push of this branch
//! starts, and runs their `run:` steps on a clean checkout of the commit -
//! the files CI gets, not the work tree's.
//!
//! The workflows are the one list of gates; nothing here keeps a second one.
//! What only GitHub can do stays there: an action is read for what it means
//! here (a toolchain, an audit) or passed over with a word, and a step that
//! publishes, needs a secret or changes the system is never run.

mod copy;
mod run;
mod shell;

use std::error::Error;
use std::path::{Path, PathBuf};

use yaml_rust2::Yaml;

use crate::cli::CiArgs;
use crate::commands::doctor::Git;
use crate::workflows::expr::{self, Scope, Value};
use crate::workflows::{self, Combination, Start, Workflow};

/// An operating system, as a runner label names one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    Linux,
    Windows,
    Mac,
}

impl Os {
    pub fn here() -> Os {
        if cfg!(windows) {
            Os::Windows
        } else if cfg!(target_os = "macos") {
            Os::Mac
        } else {
            Os::Linux
        }
    }

    /// The system a `runs-on` label stands for.
    fn of_label(label: &str) -> Option<Os> {
        let label = label.to_ascii_lowercase();
        if label.contains("windows") {
            Some(Os::Windows)
        } else if label.contains("macos") || label.contains("mac") {
            Some(Os::Mac)
        } else if label.contains("ubuntu") || label.contains("linux") {
            Some(Os::Linux)
        } else {
            None
        }
    }

    /// `runner.os`, as GitHub writes it.
    pub fn name(self) -> &'static str {
        match self {
            Os::Linux => "Linux",
            Os::Windows => "Windows",
            Os::Mac => "macOS",
        }
    }
}

/// One run of a job: a combination of its matrix on one runner.
pub struct Row {
    pub job: String,
    pub title: String,
    pub combination: Combination,
    /// The system the workflow asks for.
    pub os: Os,
    /// Run here although CI runs it on another system: the job has no row
    /// for this one.
    pub crossed: bool,
    /// The same steps as a row that runs here, on another system: CI's
    /// alone, and no gap in what this run checks.
    pub elsewhere: bool,
    /// Why it does not run here, when it does not.
    pub skip: Option<String>,
}

/// A workflow, and what a push would make of it.
pub struct Planned {
    pub workflow: Workflow,
    pub start: Start,
    pub rows: Vec<Row>,
}

pub fn run(args: CiArgs) -> Result<bool, Box<dyn Error>> {
    let path = args.path.clone().unwrap_or_else(|| PathBuf::from("."));
    let git = Git::open(&path).ok_or_else(|| {
        format!(
            "`{}` is not in a git repository - `lyrn ci` runs what a push of a commit starts",
            path.display()
        )
    })?;
    let root = git.root().to_path_buf();
    let head = git
        .run(&["rev-parse", "HEAD"])
        .map(|s| s.trim().to_string())
        .ok_or("the repository has no commit yet - CI runs on a commit")?;
    let branch = branch(&git);
    let changed = changed_files(&git);
    let workflows = committed_workflows(&git)?;
    if workflows.is_empty() {
        println!("lyrn ci: {} at {} carries no workflows - there is no CI to run", root.display(), &head[..7]);
        return Ok(true);
    }

    let mut plan: Vec<Planned> = workflows
        .into_iter()
        .map(|workflow| {
            let start = workflow.starts_on_push(&branch, changed.as_deref());
            let rows = if start == Start::Yes { rows(&workflow) } else { Vec::new() };
            Planned { workflow, start, rows }
        })
        .collect();

    if !args.jobs.is_empty() {
        let known: Vec<String> = plan.iter().flat_map(|p| p.workflow.jobs().into_iter().map(|j| j.id.to_string())).collect();
        for wanted in &args.jobs {
            if !known.contains(wanted) {
                return Err(format!("no workflow of this commit has a job `{wanted}` (its jobs: {})", known.join(", ")).into());
            }
        }
        for planned in &mut plan {
            planned.rows.retain(|row| args.jobs.contains(&row.job));
        }
    }

    let dirty = git.run(&["status", "--porcelain"]).is_some_and(|s| !s.trim().is_empty());
    println!("lyrn ci: {} at {}, as a push of `{branch}` would run it", root.display(), &head[..7]);
    if dirty {
        println!("The work tree has changes the commit does not: CI sees the commit, and so does this run.");
    }

    if args.list {
        print!("{}", list(&plan));
        return Ok(true);
    }

    let slug = crate::repo::origin_repo(&root);
    let origin = git.run(&["remote", "get-url", "origin"]).map(|s| s.trim().to_string());
    let work = copy::work_dir(&root);
    let checkout = copy::prepare(&root, &work, &head, origin.as_deref())?;
    println!("Checked out clean in {}\n", checkout.display());
    let context = run::Context {
        checkout,
        work,
        head,
        branch,
        slug,
    };
    Ok(run::all(&context, &plan))
}

/// The branch a push would be of: the one checked out, else the default.
fn branch(git: &Git) -> String {
    if let Some(branch) = git.run(&["symbolic-ref", "--quiet", "--short", "HEAD"]) {
        return branch.trim().to_string();
    }
    git.run(&["symbolic-ref", "--quiet", "--short", "refs/remotes/origin/HEAD"])
        .and_then(|r| r.trim().split_once('/').map(|(_, b)| b.to_string()))
        .unwrap_or_else(|| "main".to_string())
}

/// The files the push would change: everything the branch has that its
/// upstream does not, or - when nothing waits to be pushed - the last commit.
/// `None` when it cannot be told - a branch never pushed is all new to
/// GitHub - which no path filter can then rule out.
fn changed_files(git: &Git) -> Option<Vec<String>> {
    let list = |out: String| out.lines().map(str::to_string).filter(|l| !l.is_empty()).collect::<Vec<_>>();
    let ahead = git.run(&["rev-list", "--count", "@{u}..HEAD"]).and_then(|n| n.trim().parse::<usize>().ok())?;
    match ahead {
        n if n > 0 => git.run(&["diff", "--name-only", "@{u}...HEAD"]).map(list),
        _ => {
            let has_parent = git.run(&["rev-parse", "--verify", "--quiet", "HEAD^"]).is_some();
            if has_parent {
                git.run(&["diff-tree", "--no-commit-id", "--name-only", "-r", "HEAD"]).map(list)
            } else {
                git.run(&["ls-tree", "-r", "--name-only", "HEAD"]).map(list)
            }
        }
    }
}

/// The workflows as the commit has them, not as the work tree does.
fn committed_workflows(git: &Git) -> Result<Vec<Workflow>, Box<dyn Error>> {
    let listed = git.run(&["ls-tree", "-r", "--name-only", "HEAD", ".github/workflows"]).unwrap_or_default();
    let mut workflows = Vec::new();
    for file in listed.lines().filter(|f| workflows::is_workflow(f)) {
        let text = git.run(&["show", &format!("HEAD:{file}")]).unwrap_or_default();
        match workflows::parse(file, text) {
            Ok(workflow) => workflows.push(workflow),
            Err(reason) => return Err(format!("{file} does not parse, and GitHub would refuse it too: {reason}").into()),
        }
    }
    Ok(workflows)
}

/// Every row of every job, and whether it runs here.
fn rows(workflow: &Workflow) -> Vec<Row> {
    let here = Os::here();
    let mut rows = Vec::new();
    for job in workflow.jobs() {
        let skip_all = if job.uses.is_some() {
            Some("calls a reusable workflow, which only GitHub runs".to_string())
        } else if !matches!(job.doc["container"], Yaml::BadValue | Yaml::Null) {
            Some("runs in a container, which only GitHub starts".to_string())
        } else {
            None
        };
        let combinations = match workflows::combinations(job.doc) {
            Ok(combinations) => combinations,
            Err(reason) => {
                rows.push(Row {
                    job: job.id.to_string(),
                    title: job.id.to_string(),
                    combination: Vec::new(),
                    os: here,
                    crossed: false,
                    elsewhere: false,
                    skip: Some(reason),
                });
                continue;
            }
        };
        let mut job_rows: Vec<Row> = combinations
            .into_iter()
            .map(|combination| {
                let scope = Scope::new().with("matrix", workflows::matrix_value(&combination));
                let label = runs_on(&job.doc["runs-on"], &scope);
                let (os, skip) = match label.as_deref().and_then(Os::of_label) {
                    Some(os) => (os, None),
                    None => (
                        here,
                        Some(format!("runs on `{}`, a runner lyrn cannot stand in for", label.unwrap_or_default())),
                    ),
                };
                Row {
                    job: job.id.to_string(),
                    title: workflows::job_title(&job, &combination),
                    combination,
                    os,
                    crossed: false,
                    elsewhere: false,
                    skip: skip.or_else(|| skip_all.clone()),
                }
            })
            .collect();
        let has_here = job_rows.iter().any(|r| r.os == here);
        for row in &mut job_rows {
            if row.skip.is_some() {
                continue;
            }
            if row.os != here {
                if has_here {
                    row.elsewhere = true;
                    row.skip = Some(format!("the same steps on {} - its {} row runs here", row.os.name(), here.name()));
                } else {
                    row.crossed = true;
                }
            }
        }
        rows.extend(job_rows);
    }
    rows
}

/// A `runs-on`, as the label that names its system.
fn runs_on(value: &Yaml, scope: &Scope) -> Option<String> {
    let text = match value {
        Yaml::String(s) => s.clone(),
        Yaml::Array(labels) => labels
            .iter()
            .filter_map(workflows::scalar)
            .find(|l| Os::of_label(l).is_some())
            .or_else(|| labels.first().and_then(workflows::scalar))?,
        Yaml::Hash(_) => workflows::scalar(&value["labels"]).or_else(|| value["labels"].as_vec().and_then(|l| l.first()).and_then(workflows::scalar))?,
        _ => return None,
    };
    expr::interpolate(&text, scope).ok()
}

/// What would run, and what would not, without running anything.
fn list(plan: &[Planned]) -> String {
    let mut out = String::from("\n");
    let here = Os::here();
    for planned in plan {
        match &planned.start {
            Start::Not(reason) => {
                out.push_str(&format!("{} - not started: it {reason}\n", planned.workflow.name()));
                continue;
            }
            Start::Yes if planned.rows.is_empty() => {
                out.push_str(&format!("{} - none of its jobs was chosen\n", planned.workflow.name()));
                continue;
            }
            Start::Yes => out.push_str(&format!("{}\n", planned.workflow.name())),
        }
        let width = planned.rows.iter().filter(|r| !r.elsewhere).map(|r| r.title.chars().count()).max().unwrap_or(0) + 2;
        for row in planned.rows.iter().filter(|r| !r.elsewhere) {
            let what = match (&row.skip, row.crossed) {
                (Some(reason), _) => format!("- {reason}"),
                (None, true) => format!("runs here: a {} job, run on {}", row.os.name(), here.name()),
                (None, false) => "runs here".to_string(),
            };
            out.push_str(&format!("  {:<width$}{what}\n", row.title));
        }
        if let Some(line) = elsewhere(&planned.rows) {
            out.push_str(&line);
        }
    }
    out
}

/// The rows CI runs on other systems with the same steps as rows that run
/// here, said in one line rather than one each.
pub fn elsewhere(rows: &[Row]) -> Option<String> {
    let others: Vec<&Row> = rows.iter().filter(|r| r.elsewhere).collect();
    if others.is_empty() {
        return None;
    }
    let mut systems: Vec<&str> = others.iter().map(|r| r.os.name()).collect();
    systems.sort();
    systems.dedup();
    let count = if others.len() == 1 {
        "1 more row".to_string()
    } else {
        format!("{} more rows", others.len())
    };
    Some(format!(
        "  + {count} on {}: the same steps as the {} rows here, CI's alone\n",
        systems.join(" and "),
        Os::here().name()
    ))
}

/// The scope a job's expressions read: what GitHub would give them on a push.
pub fn job_scope(context: &run::Context, row: &Row, temp: &Path) -> Scope {
    let reference = format!("refs/heads/{}", context.branch);
    let github = Value::object([
        ("event_name", Value::string("push")),
        ("ref", Value::string(&reference)),
        ("ref_name", Value::string(&context.branch)),
        ("ref_type", Value::string("branch")),
        ("sha", Value::string(&context.head)),
        ("repository", Value::string(context.slug.clone().unwrap_or_default())),
        (
            "repository_owner",
            Value::string(context.slug.as_deref().and_then(|s| s.split('/').next()).unwrap_or_default()),
        ),
        ("workspace", Value::string(context.checkout.display().to_string())),
        ("server_url", Value::string("https://github.com")),
        ("head_ref", Value::string("")),
        ("base_ref", Value::string("")),
        ("workflow", Value::string("")),
        ("job", Value::string(&row.job)),
        ("run_id", Value::string("0")),
        ("run_number", Value::string("0")),
        ("run_attempt", Value::string("1")),
        ("actor", Value::string("")),
        ("event", Value::object(Vec::<(String, Value)>::new())),
    ]);
    let runner = Value::object([
        ("os", Value::string(row.os.name())),
        ("arch", Value::string(if cfg!(target_arch = "aarch64") { "ARM64" } else { "X64" })),
        ("temp", Value::string(temp.display().to_string())),
        ("tool_cache", Value::string(temp.display().to_string())),
        ("name", Value::string("lyrn ci")),
        ("environment", Value::string("self-hosted")),
    ]);
    Scope::new()
        .with("github", github)
        .with("runner", runner)
        .with("matrix", workflows::matrix_value(&row.combination))
        .with("strategy", Value::object([("fail-fast", Value::Bool(true))]))
        .with("job", Value::object([("status", Value::string("success"))]))
        .with("steps", Value::object(Vec::<(String, Value)>::new()))
        .with("needs", Value::object(Vec::<(String, Value)>::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows_of(yaml: &str) -> Vec<(String, Option<String>, bool)> {
        let workflow = workflows::parse("ci.yml", yaml.to_string()).unwrap();
        rows(&workflow).into_iter().map(|r| (r.title, r.skip, r.crossed)).collect()
    }

    #[test]
    fn a_job_runs_here_on_this_system_s_row_and_nowhere_else() {
        let yaml = "on: push\njobs:\n  check:\n    runs-on: ${{ matrix.os }}\n    strategy:\n      matrix:\n        os: [ubuntu-latest, windows-latest, macos-latest]\n    steps:\n      - run: cargo test\n";
        let rows = rows_of(yaml);
        assert_eq!(rows.len(), 3);
        let here: Vec<&String> = rows.iter().filter(|(_, skip, _)| skip.is_none()).map(|(title, _, _)| title).collect();
        let label = match Os::here() {
            Os::Linux => "check (ubuntu-latest)",
            Os::Windows => "check (windows-latest)",
            Os::Mac => "check (macos-latest)",
        };
        assert_eq!(here, [label]);
    }

    /// A job CI pins to one system still runs here when it has no row for
    /// this one: msrv and audit jobs are where red CI comes from, and they
    /// do not care which system builds them.
    #[test]
    fn a_job_pinned_to_another_system_runs_here_crossed() {
        let other = if Os::here() == Os::Linux { "windows-latest" } else { "ubuntu-latest" };
        let yaml = format!("on: push\njobs:\n  msrv:\n    runs-on: {other}\n    steps:\n      - run: cargo check\n");
        assert_eq!(rows_of(&yaml), [("msrv".to_string(), None, true)]);
    }

    #[test]
    fn what_only_github_runs_is_said_so() {
        let yaml = "on: push\njobs:\n  shared:\n    uses: owner/repo/.github/workflows/x.yml@v1\n  boxed:\n    runs-on: ubuntu-latest\n    container: rust:1\n    steps:\n      - run: cargo test\n  custom:\n    runs-on: [self-hosted, gpu]\n    steps:\n      - run: nvidia-smi\n";
        let rows = rows_of(yaml);
        assert!(
            rows.iter().all(|(_, skip, _)| skip.is_some()),
            "{:?}",
            rows.iter().map(|r| &r.1).collect::<Vec<_>>()
        );
    }
}
