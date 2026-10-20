//! Running the plan: each row of each job, its steps in order, the way a
//! runner would run them - and an account of what ran, what failed and what
//! only GitHub can do.

use std::fs::File;
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::{Command as Process, Stdio};
use std::time::{Duration, Instant};

use regex::Regex;
use yaml_rust2::Yaml;

use super::shell::{self, Shell};
use super::{Os, Planned, Row, job_scope};
use crate::workflows::expr::{self, Scope, Value};
use crate::workflows::{Job, Start, Step, Workflow, scalar};

/// What a run is given.
pub struct Context {
    pub checkout: PathBuf,
    pub work: PathBuf,
    pub head: String,
    pub branch: String,
    pub slug: Option<String>,
}

/// How a row of a job ended.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Ended {
    Passed,
    /// The step it failed at.
    Failed(String),
    /// A step needed a program this machine does not have: not the
    /// commit's fault, and not a pass either.
    Blocked(String),
    Skipped(String),
}

/// Runs every row the plan has for this system; whether nothing failed.
pub fn all(context: &Context, plan: &[Planned]) -> bool {
    let logs = context.work.join("logs");
    let _ = std::fs::remove_dir_all(&logs);
    let _ = std::fs::create_dir_all(&logs);
    let mut ended: Vec<(String, Ended)> = Vec::new();
    let mut counter = 0;
    let started = Instant::now();
    for planned in plan {
        if let Start::Not(reason) = &planned.start {
            println!("{} - not started: it {reason}\n", planned.workflow.name());
            continue;
        }
        if planned.rows.is_empty() {
            println!("{} - none of its jobs was chosen\n", planned.workflow.name());
            continue;
        }
        println!("{}", planned.workflow.name());
        let jobs = planned.workflow.jobs();
        // A job's rows end together: one failed row fails the job for the
        // jobs that need it.
        let mut results: Vec<(String, &'static str, Value)> = Vec::new();
        for id in in_order(&jobs) {
            let Some(job) = jobs.iter().find(|j| j.id == id) else { continue };
            let rows: Vec<&Row> = planned.rows.iter().filter(|r| r.job == id && !r.elsewhere).collect();
            if rows.is_empty() {
                continue;
            }
            let mut job_result = "success";
            let mut outputs = Value::Null;
            for row in rows {
                let label = format!("{}: {}", planned.workflow.name(), row.title);
                if let Some(reason) = &row.skip {
                    println!("  - {} - {reason}", row.title);
                    ended.push((label, Ended::Skipped(reason.clone())));
                    continue;
                }
                if let Some(need) = unmet(job, &results) {
                    let reason = format!("needs {need}, which did not pass");
                    println!("  - {} - {reason}", row.title);
                    ended.push((label, Ended::Skipped(reason)));
                    job_result = "skipped";
                    continue;
                }
                counter += 1;
                let (end, row_outputs) = run_row(context, &planned.workflow, job, row, counter, &results);
                if matches!(end, Ended::Failed(_)) {
                    job_result = "failure";
                }
                outputs = row_outputs;
                ended.push((label, end));
            }
            results.push((id.to_string(), job_result, outputs));
        }
        if let Some(line) = super::elsewhere(&planned.rows) {
            print!("{line}");
        }
        println!();
    }

    let failed: Vec<String> = ended
        .iter()
        .filter_map(|(label, end)| match end {
            Ended::Failed(step) => Some(format!("{label} (at \"{step}\")")),
            _ => None,
        })
        .collect();
    let blocked: Vec<String> = ended
        .iter()
        .filter_map(|(label, end)| match end {
            Ended::Blocked(reason) => Some(format!("{label}: {reason}")),
            _ => None,
        })
        .collect();
    let passed = ended.iter().filter(|(_, e)| *e == Ended::Passed).count();
    let skipped = ended.iter().filter(|(_, e)| matches!(e, Ended::Skipped(_))).count();
    let mut parts = vec![format!("{} passed", jobs(passed))];
    if !failed.is_empty() {
        parts.insert(0, format!("{} failed", jobs(failed.len())));
    }
    if !blocked.is_empty() {
        parts.push(format!("{} could not run here", jobs(blocked.len())));
    }
    if skipped > 0 {
        parts.push(format!("{skipped} not run here"));
    }
    println!("{} in {}", parts.join(", "), duration(started.elapsed()));
    for failure in &failed {
        println!("  ✗ {failure}");
    }
    for reason in &blocked {
        println!("  ! {reason} - install it and run again");
    }
    if counter > 0 {
        println!("Logs: {}", logs.display());
    }
    failed.is_empty() && blocked.is_empty()
}

fn jobs(n: usize) -> String {
    if n == 1 { "1 job".to_string() } else { format!("{n} jobs") }
}

/// The jobs in an order their `needs` allow, otherwise as written.
fn in_order<'a>(jobs: &[Job<'a>]) -> Vec<&'a str> {
    let mut done: Vec<&str> = Vec::new();
    while done.len() < jobs.len() {
        let before = done.len();
        for job in jobs {
            if done.contains(&job.id) {
                continue;
            }
            if needs(job).iter().all(|n| done.contains(&n.as_str()) || !jobs.iter().any(|j| j.id == n)) {
                done.push(job.id);
            }
        }
        if done.len() == before {
            // A cycle GitHub would refuse; run the rest as written.
            done.extend(jobs.iter().map(|j| j.id).filter(|id| !done.contains(id)).collect::<Vec<_>>());
        }
    }
    done
}

fn needs(job: &Job) -> Vec<String> {
    match &job.doc["needs"] {
        Yaml::Array(items) => items.iter().filter_map(scalar).collect(),
        other => scalar(other).into_iter().collect(),
    }
}

/// A job this one needs that ran and did not pass.
fn unmet(job: &Job, results: &[(String, &'static str, Value)]) -> Option<String> {
    needs(job)
        .into_iter()
        .find(|need| results.iter().any(|(id, result, _)| id == need && *result != "success"))
}

/// One row of a job, step by step.
fn run_row(context: &Context, workflow: &Workflow, job: &Job, row: &Row, number: usize, results: &[(String, &'static str, Value)]) -> (Ended, Value) {
    let temp = context.work.join("temp").join(number.to_string());
    let _ = std::fs::remove_dir_all(&temp);
    let _ = std::fs::create_dir_all(&temp);
    let crossed = if row.crossed {
        format!(" - a {} job, run here on {}", row.os.name(), Os::here().name())
    } else {
        String::new()
    };
    println!("  {}{crossed}", row.title);
    let started = Instant::now();

    let mut scope = job_scope(context, row, &temp);
    scope.set(
        "needs",
        Value::Object(
            results
                .iter()
                .map(|(id, result, outputs)| (id.clone(), Value::object([("result", Value::string(*result)), ("outputs", outputs.clone())])))
                .collect(),
        ),
    );
    if let Some(services) = job.doc["services"].as_hash() {
        let names: Vec<String> = services.keys().filter_map(scalar).collect();
        println!(
            "    · its {} service is not started here: the steps reach whatever answers where they look",
            names.join(", ")
        );
    }

    let mut state = State {
        env: base_env(context, row, &temp),
        path: Vec::new(),
        steps: Vec::new(),
        toolchain_file: context.checkout.join("rust-toolchain.toml").is_file() || context.checkout.join("rust-toolchain").is_file(),
    };
    for (key, value) in env_of(&workflow.doc()["env"], &mut scope, &state)
        .into_iter()
        .chain(env_of(&job.doc["env"], &mut scope, &state))
    {
        state.set(key, value);
    }
    scope.set("env", state.env_value());
    match expr::condition(scalar(&job.doc["if"]).as_deref(), &scope) {
        Ok(true) => {}
        Ok(false) => {
            println!("    - its condition is false on this push");
            return (Ended::Skipped("its condition is false on this push".to_string()), Value::Null);
        }
        Err(reason) => {
            println!("    - its condition {reason}");
            return (Ended::Skipped(format!("its condition {reason}")), Value::Null);
        }
    }

    let mut failed_at: Option<String> = None;
    let mut blocked: Option<String> = None;
    let mut ran_any = false;
    // Steps that did not run here, whose outputs nothing after them can use.
    let mut unavailable: Vec<String> = Vec::new();
    for (index, step) in job.steps.iter().enumerate() {
        scope.failed = failed_at.is_some();
        scope.set("steps", Value::Object(state.steps.clone()));
        scope.set("env", state.env_value());
        let title = expr::interpolate(&step.title(), &scope).unwrap_or_else(|_| step.title());
        match expr::condition(scalar(&step.doc["if"]).as_deref(), &scope) {
            Ok(true) => {}
            Ok(false) => {
                if failed_at.is_none() {
                    println!("    - {title} - its condition is false here");
                }
                state.record(step, "skipped", Value::Null);
                continue;
            }
            Err(reason) => {
                println!("    - {title} - its condition {reason}");
                state.record(step, "skipped", Value::Null);
                continue;
            }
        }
        let step_env = env_of(&step.doc["env"], &mut scope, &state);
        let log = context
            .work
            .join("logs")
            .join(format!("{number:02}-{:02}-{}.log", index + 1, file_name(&title)));
        let outcome = if blocked.is_some() {
            // What follows a step this machine could not stand in for would
            // check something other than CI does: a green there means nothing.
            Outcome::Skip("not run: a step before it could not run here".to_string())
        } else if let Some(id) = unavailable.iter().find(|id| uses_outputs_of(step, id)) {
            Outcome::Skip(format!("uses what step `{id}` gives, and that did not run here"))
        } else if let Some(uses) = step.uses {
            action(context, step, uses, &scope, &mut state, &step_env, &log, &title)
        } else if let Some(script) = step.run {
            script_step(context, job, step, script, row.os, &scope, &mut state, &step_env, &temp, index, &log, &title)
        } else {
            Outcome::Read("nothing to run".to_string())
        };
        // A program that was not found blocks the step whatever its exit
        // code: `echo "v=$(jq …)"` succeeds and hands on an empty value.
        let outcome = match outcome {
            Outcome::Ran { took, .. } if missing_program(&log).is_some() => Outcome::Blocked {
                program: missing_program(&log).unwrap_or_default(),
                took,
            },
            other => other,
        };
        let ok = match &outcome {
            Outcome::Ran { ok, .. } => *ok,
            _ => true,
        };
        if matches!(outcome, Outcome::Ran { .. }) {
            ran_any = true;
        }
        if let Some(id) = step.doc["id"].as_str()
            && matches!(outcome, Outcome::Skip(_) | Outcome::Blocked { .. } | Outcome::Unsure(_))
        {
            unavailable.push(id.to_string());
        }
        if blocked.is_none() {
            match &outcome {
                Outcome::Blocked { program, .. } => blocked = Some(format!("\"{title}\" needs {program}, which is not installed here")),
                Outcome::Unsure(reason) => blocked = Some(format!("\"{title}\" {reason}")),
                _ => {}
            }
        }
        report(&title, &outcome, &log);
        let continue_on_error = step.doc["continue-on-error"].as_bool() == Some(true)
            || scalar(&step.doc["continue-on-error"]).is_some_and(|t| expr::condition(Some(&t), &scope).unwrap_or(false));
        let (result, outputs) = match &outcome {
            Outcome::Ran { outputs, .. } => (if ok { "success" } else { "failure" }, Value::Object(outputs.clone())),
            Outcome::Skip(_) => ("skipped", Value::Null),
            _ => ("success", Value::Null),
        };
        state.record(step, result, outputs);
        if !ok && !continue_on_error && failed_at.is_none() {
            failed_at = Some(title);
        }
    }

    let outputs = job.doc["outputs"]
        .as_hash()
        .map(|outputs| {
            scope.set("steps", Value::Object(state.steps.clone()));
            Value::Object(
                outputs
                    .iter()
                    .filter_map(|(k, v)| Some((scalar(k)?, Value::string(expr::interpolate(&scalar(v)?, &scope).unwrap_or_default()))))
                    .collect(),
            )
        })
        .unwrap_or(Value::Null);
    let took = duration(started.elapsed());
    match (failed_at, blocked) {
        (Some(step), _) => {
            println!("  ✗ {} failed at \"{step}\", {took}", row.title);
            (Ended::Failed(step), outputs)
        }
        (None, Some(reason)) => {
            println!("  ! {} could not run here: {reason}, {took}", row.title);
            (Ended::Blocked(reason), outputs)
        }
        (None, None) if !ran_any => {
            println!("  - {} ran nothing: none of its steps runs here", row.title);
            (Ended::Skipped("none of its steps runs here".to_string()), outputs)
        }
        (None, None) => {
            println!("  ✓ {} passed, {took}", row.title);
            (Ended::Passed, outputs)
        }
    }
}

/// What a row carries from one step to the next.
struct State {
    env: Vec<(String, String)>,
    /// Directories `$GITHUB_PATH` added, newest first.
    path: Vec<String>,
    /// The `steps` context: each step with an id, its outcome and outputs.
    steps: Vec<(String, Value)>,
    /// A rust-toolchain file decides the toolchain, here as on CI.
    toolchain_file: bool,
}

impl State {
    fn set(&mut self, key: String, value: String) {
        match self.env.iter_mut().find(|(k, _)| k.eq_ignore_ascii_case(&key)) {
            Some((_, existing)) => *existing = value,
            None => self.env.push((key, value)),
        }
    }

    fn env_value(&self) -> Value {
        Value::Object(self.env.iter().map(|(k, v)| (k.clone(), Value::string(v))).collect())
    }

    fn record(&mut self, step: &Step, result: &str, outputs: Value) {
        let Some(id) = step.doc["id"].as_str() else { return };
        let outputs = if outputs == Value::Null { Value::Object(Vec::new()) } else { outputs };
        self.steps.push((
            id.to_string(),
            Value::object([("outputs", outputs), ("outcome", Value::string(result)), ("conclusion", Value::string(result))]),
        ));
    }

    /// The process environment for a step: the row's, the step's own, and
    /// PATH with what earlier steps added in front.
    fn process_env(&self, step_env: &[(String, String)]) -> Vec<(String, String)> {
        let mut env = self.env.clone();
        for (k, v) in step_env {
            match env.iter_mut().find(|(key, _)| key.eq_ignore_ascii_case(k)) {
                Some((_, existing)) => *existing = v.clone(),
                None => env.push((k.clone(), v.clone())),
            }
        }
        if !self.path.is_empty() {
            let current = std::env::var_os("PATH").unwrap_or_default();
            let joined =
                std::env::join_paths(self.path.iter().map(PathBuf::from).chain(std::env::split_paths(&current))).map(|p| p.to_string_lossy().into_owned());
            if let Ok(joined) = joined {
                env.push(("PATH".to_string(), joined));
            }
        }
        env
    }
}

/// What GitHub's runner puts in every step's environment.
fn base_env(context: &Context, row: &Row, temp: &Path) -> Vec<(String, String)> {
    let mut env = vec![
        ("CI".to_string(), "true".to_string()),
        ("GITHUB_ACTIONS".to_string(), "true".to_string()),
        ("GITHUB_WORKSPACE".to_string(), context.checkout.display().to_string()),
        ("GITHUB_SHA".to_string(), context.head.clone()),
        ("GITHUB_REF".to_string(), format!("refs/heads/{}", context.branch)),
        ("GITHUB_REF_NAME".to_string(), context.branch.clone()),
        ("GITHUB_REF_TYPE".to_string(), "branch".to_string()),
        ("GITHUB_EVENT_NAME".to_string(), "push".to_string()),
        ("GITHUB_SERVER_URL".to_string(), "https://github.com".to_string()),
        ("GITHUB_JOB".to_string(), row.job.clone()),
        ("RUNNER_OS".to_string(), row.os.name().to_string()),
        (
            "RUNNER_ARCH".to_string(),
            if cfg!(target_arch = "aarch64") { "ARM64" } else { "X64" }.to_string(),
        ),
        ("RUNNER_TEMP".to_string(), temp.display().to_string()),
    ];
    if let Some(slug) = &context.slug {
        env.push(("GITHUB_REPOSITORY".to_string(), slug.clone()));
    }
    env
}

/// An `env:` mapping, its values with their expressions filled in.
fn env_of(value: &Yaml, scope: &mut Scope, state: &State) -> Vec<(String, String)> {
    let Some(map) = value.as_hash() else { return Vec::new() };
    let mut out: Vec<(String, String)> = Vec::new();
    for (key, value) in map {
        let (Some(key), Some(value)) = (scalar(key), scalar(value)) else { continue };
        // An expression nobody here can answer leaves the variable unset,
        // as an empty secret would on a fork.
        if let Ok(value) = expr::interpolate(&value, scope) {
            out.push((key, value));
        }
    }
    let mut all = state.env.clone();
    all.extend(out.iter().cloned());
    scope.set("env", Value::Object(all.into_iter().map(|(k, v)| (k, Value::String(v))).collect()));
    out
}

/// What a step came to.
enum Outcome {
    /// Something ran here.
    Ran {
        ok: bool,
        note: Option<String>,
        took: Duration,
        outputs: Vec<(String, Value)>,
    },
    /// An action read for what it means here, with nothing to run.
    Read(String),
    /// Read, with something worth a look.
    Note(String),
    /// Not run here, and why.
    Skip(String),
    /// Run, and stopped by a program this machine does not have.
    Blocked { program: String, took: Duration },
    /// Read, and found to need something this run cannot stand in for: the
    /// steps after it would check something else than CI does.
    Unsure(String),
}

fn report(title: &str, outcome: &Outcome, log: &Path) {
    match outcome {
        Outcome::Ran { ok: true, note, took, .. } => {
            let note = once(note).map_or(String::new(), |n| format!(" ({n})"));
            println!("    ✓ {title}  {}{note}", duration(*took));
        }
        Outcome::Ran { ok: false, note, took, .. } => {
            let note = once(note).map_or(String::new(), |n| format!(" ({n})"));
            println!("    ✗ {title}  {}{note}", duration(*took));
            let text = std::fs::read_to_string(log).unwrap_or_default();
            let lines: Vec<&str> = text.lines().collect();
            for line in &lines[lines.len().saturating_sub(30)..] {
                println!("        {line}");
            }
            println!("        whole log: {}", log.display());
        }
        Outcome::Read(what) => println!("    · {title} - {what}"),
        Outcome::Note(what) => println!("    ! {title} - {what}"),
        Outcome::Skip(why) => println!("    - {title} - {why}"),
        Outcome::Unsure(reason) => println!("    ! {title} - {reason}"),
        Outcome::Blocked { program, took } => {
            println!(
                "    ! {title}  {} - needs {program}, which is not installed here; the commit is not at fault",
                duration(*took)
            );
            println!("        whole log: {}", log.display());
        }
    }
}

/// A note said the first time it comes up, and not again: one that holds
/// for the whole run - which PowerShell runs the scripts - is news once.
fn once(note: &Option<String>) -> Option<&String> {
    static SAID: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
    let note = note.as_ref()?;
    let mut said = SAID.lock().ok()?;
    if said.contains(note) {
        return None;
    }
    said.push(note.clone());
    Some(note)
}

/// The program a step could not find, by what each shell says then.
fn missing_program(log: &Path) -> Option<String> {
    let text = std::fs::read_to_string(log).ok()?;
    let patterns = [
        r"(?m)(?:^|: )([\w.+-]+): command not found",
        r"The term '([^']+)' is not recognized",
        r"'([^']+)' is not recognized as an internal or external command",
        r"(?m)^\S+: \d+: ([\w.+-]+): not found",
    ];
    patterns
        .iter()
        .find_map(|pattern| Regex::new(pattern).expect("a valid pattern").captures(&text).map(|c| format!("`{}`", &c[1])))
}

/// Whether a step reads the outputs of the step `id`.
fn uses_outputs_of(step: &Step, id: &str) -> bool {
    let needle = Regex::new(&format!(r"\bsteps\.{}\.", regex::escape(id))).expect("a valid pattern");
    let mut texts: Vec<String> = Vec::new();
    texts.extend(step.run.map(str::to_string));
    texts.extend(step.working_directory.map(str::to_string));
    for key in ["with", "env"] {
        if let Some(map) = step.doc[key].as_hash() {
            texts.extend(map.values().filter_map(scalar));
        }
    }
    texts.iter().any(|t| needle.is_match(t))
}

/// Steps that reach past this machine, and steps that change it.
fn screen(script: &str) -> Option<&'static str> {
    let publishes = Regex::new(
        r"(?m)\b(?:cargo|npm|pnpm|yarn|bun)\s+publish\b|\bgh\s+(?:release|api|pr|issue|repo|workflow|secret)\b|\bgit\s+push\b|\bdocker\s+(?:push|login)\b|\bvsce\s+publish\b|\btwine\s+upload\b|\bwrangler\s+(?:deploy|publish)\b|\bsemantic-release\b|\bchangeset\s+publish\b",
    )
    .expect("a valid pattern");
    let system =
        Regex::new(r"(?m)^\s*(?:sudo\s|apt-get\s|apt\s|brew\s+install|choco\s+install|winget\s+install|snap\s+install|dnf\s|yum\s)").expect("a valid pattern");
    if publishes.is_match(script) {
        Some("publishes past this machine - lyrn ci never runs that")
    } else if system.is_match(script) {
        Some("changes the system (packages, sudo) - not done here")
    } else {
        None
    }
}

/// Whether a script installs lyrn from its release, as the line's shopfront
/// workflow does - and the version it pins, if it pins one.
fn installs_lyrn(script: &str) -> Option<Option<String>> {
    let installer = Regex::new(r"lacodda/lyrn/[^\s/]+/tools/install\.(?:sh|ps1)").expect("a valid pattern");
    if !installer.is_match(script) {
        return None;
    }
    let pinned = Regex::new(r"LYRN_VERSION\s*=\s*['\x22]?v?([0-9][0-9A-Za-z.\-]*)").expect("a valid pattern");
    Some(pinned.captures(script).map(|c| c[1].to_string()))
}

/// The lyrn running this is a lyrn, and its directory goes in front of PATH
/// the way the installer's would: the steps after it run this one rather
/// than a download that, on Windows, the shell installer refuses anyway.
fn stand_in_for_lyrn(state: &mut State, pinned: Option<&str>) -> Outcome {
    let here = env!("CARGO_PKG_VERSION");
    match std::env::current_exe().ok().and_then(|exe| exe.parent().map(Path::to_path_buf)) {
        Some(dir) => state.path.insert(0, dir.display().to_string()),
        None => return Outcome::Unsure("installs lyrn, and the lyrn running this cannot say where it is".to_string()),
    }
    match pinned {
        Some(version) if version != here => Outcome::Note(format!("installs lyrn {version}; the steps after it run this one, {here}")),
        _ => Outcome::Read(format!("installs lyrn {here}, which is the one running this")),
    }
}

#[allow(clippy::too_many_arguments)]
fn script_step(
    context: &Context,
    job: &Job,
    step: &Step,
    script: &str,
    os: Os,
    scope: &Scope,
    state: &mut State,
    step_env: &[(String, String)],
    temp: &Path,
    index: usize,
    log: &Path,
    title: &str,
) -> Outcome {
    let body = match expr::interpolate(script, scope) {
        Ok(body) => body,
        Err(reason) => return Outcome::Skip(format!("its script {reason}")),
    };
    if let Some(reason) = screen(&body) {
        return Outcome::Skip(reason.to_string());
    }
    if let Some(pinned) = installs_lyrn(&body) {
        return stand_in_for_lyrn(state, pinned.as_deref());
    }
    let named = step.doc["shell"].as_str().or(job.shell);
    let shell = Shell::of(named, os);
    let files = temp.join("files");
    let _ = std::fs::create_dir_all(&files);
    let script_path = files.join(format!("step-{}.{}", index + 1, shell.extension()));
    if let Err(e) = std::fs::write(&script_path, shell.script(&body)) {
        return Outcome::Ran {
            ok: false,
            note: Some(format!("could not write its script: {e}")),
            took: Duration::ZERO,
            outputs: Vec::new(),
        };
    }
    let (command, note) = match shell.command(&script_path) {
        Ok(found) => found,
        Err(reason) => return Outcome::Skip(reason),
    };
    let directory = step.run_directory(job).map(|d| expr::interpolate(d, scope));
    let cwd = match directory {
        None => context.checkout.clone(),
        Some(Ok(dir)) => {
            let dir = PathBuf::from(&dir);
            if dir.is_absolute() { dir } else { context.checkout.join(dir) }
        }
        Some(Err(reason)) => return Outcome::Skip(format!("its directory {reason}")),
    };
    run_command(command, &cwd, state, step_env, &files, index, log, title, note)
}

/// Runs a command as a step, with the files a step writes back through.
#[allow(clippy::too_many_arguments)]
fn run_command(
    mut command: Process,
    cwd: &Path,
    state: &mut State,
    step_env: &[(String, String)],
    files: &Path,
    index: usize,
    log: &Path,
    title: &str,
    note: Option<String>,
) -> Outcome {
    let _ = std::fs::create_dir_all(files);
    let output = files.join(format!("output-{}", index + 1));
    let env_file = files.join(format!("env-{}", index + 1));
    let path_file = files.join(format!("path-{}", index + 1));
    let summary = files.join(format!("summary-{}", index + 1));
    for file in [&output, &env_file, &path_file, &summary] {
        let _ = File::create(file);
    }
    let mut env = state.process_env(step_env);
    env.push(("GITHUB_OUTPUT".to_string(), output.display().to_string()));
    env.push(("GITHUB_ENV".to_string(), env_file.display().to_string()));
    env.push(("GITHUB_PATH".to_string(), path_file.display().to_string()));
    env.push(("GITHUB_STEP_SUMMARY".to_string(), summary.display().to_string()));

    let progress = std::io::stdout().is_terminal();
    if progress {
        print!("    … {title}");
        let _ = std::io::stdout().flush();
    }
    let started = Instant::now();
    let result = (|| -> std::io::Result<bool> {
        let out = File::create(log)?;
        let err = out.try_clone()?;
        command
            .current_dir(cwd)
            .envs(env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
            .stdin(Stdio::null())
            .stdout(out)
            .stderr(err);
        Ok(command.status()?.success())
    })();
    if progress {
        print!("\r");
    }
    let took = started.elapsed();
    let ok = match result {
        Ok(ok) => ok,
        Err(e) => {
            let _ = std::fs::write(log, format!("could not start: {e}\n(in {})\n", cwd.display()));
            false
        }
    };
    for (key, value) in pairs(&std::fs::read_to_string(&env_file).unwrap_or_default()) {
        state.set(key, value);
    }
    for dir in std::fs::read_to_string(&path_file)
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
    {
        state.path.insert(0, dir.to_string());
    }
    let outputs = pairs(&std::fs::read_to_string(&output).unwrap_or_default())
        .into_iter()
        .map(|(k, v)| (k, Value::String(v)))
        .collect();
    Outcome::Ran { ok, note, took, outputs }
}

/// `$GITHUB_OUTPUT` and `$GITHUB_ENV`: `name=value` lines, and
/// `name<<DELIMITER` blocks for values of more than one line.
fn pairs(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut lines = text.lines();
    while let Some(line) = lines.next() {
        if let Some((name, delimiter)) = line.split_once("<<").filter(|(n, _)| !n.contains('=')) {
            let mut value = Vec::new();
            for next in lines.by_ref() {
                if next == delimiter {
                    break;
                }
                value.push(next);
            }
            out.push((name.to_string(), value.join("\n")));
        } else if let Some((name, value)) = line.split_once('=') {
            out.push((name.to_string(), value.to_string()));
        }
    }
    out
}

/// An action, read for what it means on this machine.
#[allow(clippy::too_many_arguments)]
fn action(context: &Context, step: &Step, uses: &str, scope: &Scope, state: &mut State, step_env: &[(String, String)], log: &Path, title: &str) -> Outcome {
    let (name, reference) = uses.split_once('@').unwrap_or((uses, ""));
    let name = name.to_ascii_lowercase();
    let input = |key: &str| {
        step.input(key)
            .map(|v| expr::interpolate(&v, scope).unwrap_or(v))
            .filter(|v| !v.trim().is_empty())
    };
    let base = name.split('/').take(2).collect::<Vec<_>>().join("/");
    match base.as_str() {
        "actions/checkout" => Outcome::Read("the commit is checked out clean".to_string()),
        "actions/cache" | "swatinem/rust-cache" | "mozilla-actions/sccache-action" => {
            Outcome::Read("nothing to cache - the copy keeps target/ and node_modules/".to_string())
        }
        "actions/upload-artifact" => Outcome::Read("the files stay in the copy".to_string()),
        "actions/download-artifact" => Outcome::Skip("downloads what another run uploaded, which only GitHub keeps".to_string()),
        "dtolnay/rust-toolchain" | "actions-rust-lang/setup-rust-toolchain" => {
            if step.input("toolchain").is_some() && input("toolchain").is_none() {
                return Outcome::Unsure("asks for a toolchain that came out empty here, so the steps after it would build with another".to_string());
            }
            let toolchain = input("toolchain").unwrap_or_else(|| {
                let channel = reference.starts_with(|c: char| c.is_ascii_digit()) || ["stable", "beta", "nightly"].iter().any(|c| reference.starts_with(c));
                if channel { reference.to_string() } else { "stable".to_string() }
            });
            let components: Vec<String> = input("components")
                .map(|c| c.split([',', ' ']).filter(|s| !s.is_empty()).map(str::to_string).collect())
                .unwrap_or_default();
            toolchain_step(context, state, &toolchain, &components, log, title)
        }
        "actions/setup-node" => setup("Node", input("node-version"), local_version("node")),
        "pnpm/action-setup" => setup("pnpm", input("version"), local_version("pnpm")),
        "embarkstudios/cargo-deny-action" => {
            if shell::find("cargo-deny").is_none() {
                return Outcome::Skip("cargo-deny is not installed here - `cargo install cargo-deny --locked` lets this step run".to_string());
            }
            let mut command = Process::new("cargo");
            command.args(["deny", "--manifest-path", &input("manifest-path").unwrap_or_else(|| "./Cargo.toml".to_string())]);
            command.args(["--log-level", &input("log-level").unwrap_or_else(|| "warn".to_string())]);
            command.args(input("arguments").unwrap_or_else(|| "--all-features".to_string()).split_whitespace());
            command.args(input("command").unwrap_or_else(|| "check".to_string()).split_whitespace());
            command.args(input("command-arguments").unwrap_or_default().split_whitespace());
            let files = context.work.join("temp").join("deny");
            run_command(command, &context.checkout, state, step_env, &files, 0, log, title, None)
        }
        "taiki-e/install-action" => {
            let tools: Vec<String> = input("tool")
                .map(|t| {
                    t.split([',', ' ', '\n'])
                        .filter(|s| !s.is_empty())
                        .map(|s| s.split('@').next().unwrap_or(s).to_string())
                        .collect()
                })
                .unwrap_or_default();
            let missing: Vec<&String> = tools
                .iter()
                .filter(|t| shell::find(t).is_none() && shell::find(&format!("cargo-{}", t.trim_start_matches("cargo-"))).is_none())
                .collect();
            if missing.is_empty() {
                Outcome::Read(format!("{} installed here", tools.join(", ")))
            } else {
                Outcome::Note(format!(
                    "{} not installed here; the steps that use it will fail",
                    missing.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
                ))
            }
        }
        _ if base.starts_with("actions/setup-") => Outcome::Read(format!("{} as installed here", base.trim_start_matches("actions/setup-"))),
        _ => Outcome::Skip("an action only GitHub runs".to_string()),
    }
}

/// A tool a job sets up, beside the one installed here. A job that names
/// only a major gets the newest of it on CI, which a machine may well be
/// behind: that is worth a look before a step fails on it.
fn setup(tool: &str, wanted: Option<String>, here: Option<String>) -> Outcome {
    let Some(here) = here else {
        return Outcome::Note(format!("{tool} is not installed here"));
    };
    let Some(wanted) = wanted else {
        return Outcome::Read(format!("{tool} {here} here"));
    };
    let bare = |v: &str| v.trim().trim_start_matches('v').trim_end_matches(".x").to_string();
    let (wanted_bare, here_bare) = (bare(&wanted), bare(&here));
    if here_bare == wanted_bare {
        Outcome::Read(format!("{tool} {here} here, as the job sets up"))
    } else if here_bare.starts_with(&format!("{wanted_bare}.")) {
        Outcome::Note(format!("{tool} {here} here; the job sets up the newest {wanted_bare}, which may be newer"))
    } else {
        Outcome::Note(format!("{tool} {here} here; the job sets up {wanted}"))
    }
}

/// The toolchain a job asks for: installed when it is a version this machine
/// lacks, as the runner installs it, and then used by the job's later steps -
/// unless the repository's rust-toolchain file decides, as it does on CI.
fn toolchain_step(context: &Context, state: &mut State, toolchain: &str, components: &[String], log: &Path, title: &str) -> Outcome {
    if state.toolchain_file {
        return Outcome::Read(format!("{toolchain} asked; the repository's rust-toolchain file decides, here as on CI"));
    }
    let installed = Process::new("rustup")
        .args(["toolchain", "list"])
        .output()
        .map(|out| String::from_utf8_lossy(&out.stdout).lines().any(|l| l.starts_with(&format!("{toolchain}-"))))
        .unwrap_or(false);
    let mut note = None;
    if !installed {
        let mut install = Process::new("rustup");
        install.args(["toolchain", "install", toolchain, "--profile", "minimal", "--no-self-update"]);
        if !components.is_empty() {
            install.arg("--component").arg(components.join(","));
        }
        let files = context.work.join("temp").join("toolchain");
        let outcome = run_command(
            install,
            &context.checkout,
            state,
            &[],
            &files,
            0,
            log,
            title,
            Some(format!("installed {toolchain}, as the runner does")),
        );
        if let Outcome::Ran { ok: false, .. } = outcome {
            return outcome;
        }
        note = Some(format!("installed {toolchain}"));
    } else if !components.is_empty() {
        let _ = Process::new("rustup")
            .args(["component", "add", "--toolchain", toolchain])
            .args(components)
            .output();
    }
    state.set("RUSTUP_TOOLCHAIN".to_string(), toolchain.to_string());
    match note {
        Some(note) => Outcome::Read(format!("{note}; the job's steps build with it")),
        None => Outcome::Read(format!("{toolchain}, as installed here")),
    }
}

/// `node --version`, `pnpm --version`: the version a program says it is.
fn local_version(program: &str) -> Option<String> {
    let path = shell::find(program)?;
    let out = Process::new(path).arg("--version").output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!text.is_empty()).then_some(text)
}

fn duration(d: Duration) -> String {
    let seconds = d.as_secs();
    if seconds < 60 {
        format!("{seconds}s")
    } else {
        format!("{}m {}s", seconds / 60, seconds % 60)
    }
}

/// A title as part of a file name.
fn file_name(title: &str) -> String {
    let name: String = title
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect();
    let name = name.split('-').filter(|p| !p.is_empty()).collect::<Vec<_>>().join("-");
    name.chars().take(40).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_reaches_past_this_machine_is_never_run() {
        assert!(screen("cargo publish --package demo").is_some());
        assert!(screen("pnpm install\npnpm publish --no-git-checks").is_some());
        assert!(screen("git push origin main").is_some());
        assert!(screen("gh release create v1").is_some());
        assert!(screen("sudo apt-get update && sudo apt-get install -y x").is_some());
        assert!(screen("cargo test\ncargo package --package demo-core").is_none());
        assert!(screen("cargo fmt --check").is_none());
    }

    /// The shopfront workflow installs lyrn from a release; here the lyrn
    /// running the workflow stands in for it, and says when it is not the
    /// version pinned.
    #[test]
    fn the_lyrn_installer_is_known_and_its_pin_read() {
        let script = "curl -fsSL https://raw.githubusercontent.com/lacodda/lyrn/v2.14.0/tools/install.sh | LYRN_VERSION=v2.14.0 sh\necho \"$HOME/.local/bin\" >> \"$GITHUB_PATH\"";
        assert_eq!(installs_lyrn(script), Some(Some("2.14.0".to_string())));
        assert_eq!(
            installs_lyrn("irm https://raw.githubusercontent.com/lacodda/lyrn/main/tools/install.ps1 | iex"),
            Some(None)
        );
        assert_eq!(installs_lyrn("curl -fsSL https://example.com/install.sh | sh"), None);

        let mut state = State {
            env: Vec::new(),
            path: Vec::new(),
            steps: Vec::new(),
            toolchain_file: false,
        };
        let said = match stand_in_for_lyrn(&mut state, Some("0.0.1")) {
            Outcome::Note(t) => t,
            _ => panic!("another version than the one running is worth a look"),
        };
        assert!(said.contains("0.0.1") && said.contains(env!("CARGO_PKG_VERSION")), "{said}");
        let exe_dir = std::env::current_exe().unwrap().parent().unwrap().display().to_string();
        assert_eq!(state.path.first(), Some(&exe_dir), "the steps after it do not find this lyrn first");
        assert!(matches!(stand_in_for_lyrn(&mut state, Some(env!("CARGO_PKG_VERSION"))), Outcome::Read(_)));
    }

    #[test]
    fn a_step_s_outputs_are_read_as_the_runner_reads_them() {
        let text = "version=1.88\nnotes<<EOF\nfirst\nsecond\nEOF\nempty=\n";
        assert_eq!(
            pairs(text),
            [
                ("version".to_string(), "1.88".to_string()),
                ("notes".to_string(), "first\nsecond".to_string()),
                ("empty".to_string(), String::new())
            ]
        );
    }

    #[test]
    fn a_missing_program_is_told_from_a_failure() {
        let dir = tempfile::tempdir().unwrap();
        let log = dir.path().join("step.log");
        for (text, program) in [
            ("C:/x/step-1.sh: line 1: jq: command not found\n", Some("`jq`")),
            ("jq : The term 'jq' is not recognized as the name of a cmdlet, function", Some("`jq`")),
            ("'jq' is not recognized as an internal or external command,", Some("`jq`")),
            ("/tmp/step-1.sh: 1: jq: not found\n", Some("`jq`")),
            ("error[E0425]: cannot find value `x` in this scope\n", None),
        ] {
            std::fs::write(&log, text).unwrap();
            assert_eq!(missing_program(&log).as_deref(), program, "{text}");
        }
    }

    #[test]
    fn a_step_reading_another_s_outputs_is_found() {
        let workflow = crate::workflows::parse(
            "ci.yml",
            "jobs:\n  msrv:\n    steps:\n      - id: msrv\n        run: echo version=1 >> $GITHUB_OUTPUT\n      - uses: dtolnay/rust-toolchain@master\n        with:\n          toolchain: ${{ steps.msrv.outputs.version }}\n      - run: cargo check\n".to_string(),
        )
        .unwrap();
        let jobs = workflow.jobs();
        assert!(uses_outputs_of(&jobs[0].steps[1], "msrv"));
        assert!(!uses_outputs_of(&jobs[0].steps[2], "msrv"));
    }

    #[test]
    fn a_tool_set_up_by_its_major_alone_is_worth_a_look() {
        let said = |o: Outcome| match o {
            Outcome::Read(t) => format!("· {t}"),
            Outcome::Note(t) => format!("! {t}"),
            _ => String::new(),
        };
        assert_eq!(
            said(setup("Node", Some("22.18.0".into()), Some("v22.18.0".into()))),
            "· Node v22.18.0 here, as the job sets up"
        );
        assert_eq!(
            said(setup("Node", Some("22".into()), Some("v22.18.0".into()))),
            "! Node v22.18.0 here; the job sets up the newest 22, which may be newer"
        );
        assert_eq!(
            said(setup("pnpm", Some("10".into()), Some("9.1.0".into()))),
            "! pnpm 9.1.0 here; the job sets up 10"
        );
    }

    #[test]
    fn jobs_run_after_the_jobs_they_need() {
        let workflow = crate::workflows::parse(
            "ci.yml",
            "jobs:\n  release:\n    needs: [build, test]\n  test:\n    needs: build\n  build: {}\n".to_string(),
        )
        .unwrap();
        assert_eq!(in_order(&workflow.jobs()), ["build", "test", "release"]);
    }
}
