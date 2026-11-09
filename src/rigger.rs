//! rigger, the record of the line: which repositories are yours, what stage
//! each one is in, and a line left in the record when a ritual runs.
//!
//! Which projects are yours is written down once, in rigger, with where each
//! lives and which are out of the line; walking a directory would be a second
//! answer to that question, and it would pick up every clone and experiment
//! that happens to sit there. lyrn asks through rigger's command line, the
//! way a person would, and keeps no copy of what it is told.

use std::error::Error;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command as Process;

use serde_json::Value;

/// The program asked: `rigger`, unless `LYRN_RIGGER` names another - which
/// only the tests should, to stand a record of their own in for yours.
fn program() -> OsString {
    std::env::var_os("LYRN_RIGGER").filter(|v| !v.is_empty()).unwrap_or_else(|| "rigger".into())
}

/// `rigger <args>`; its standard output, or why there is none.
fn ask(args: &[&str]) -> Result<String, String> {
    let out = Process::new(program()).args(args).output().map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            "rigger is not installed here".to_string()
        } else {
            format!("rigger could not be run: {error}")
        }
    })?;
    if !out.status.success() {
        return Err(format!("`rigger {}` failed: {}", args.join(" "), String::from_utf8_lossy(&out.stderr).trim()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// A project the record keeps but a command leaves alone, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skipped {
    pub name: String,
    pub reason: String,
}

/// The line's repositories by name, and the projects passed over.
pub type Line = (Vec<(String, PathBuf)>, Vec<Skipped>);

/// Every repository of the line, by name, and the projects passed over.
pub fn line() -> Result<Line, Box<dyn Error>> {
    let json = ask(&["--json", "project", "list"]).map_err(|reason| {
        if reason.contains("not installed") {
            "`--all` reads your projects from rigger's record, and rigger is not installed here - name the repositories instead".to_string()
        } else {
            reason
        }
    })?;
    parse_line(&json)
}

/// What `rigger --json project list` prints, read into repositories: a
/// project with a repository is taken, one the record keeps out of the line
/// is passed over by name, and a place the record keeps for itself is not a
/// project anyone holds to a standard.
fn parse_line(json: &str) -> Result<Line, Box<dyn Error>> {
    let projects: Value = serde_json::from_str(json).map_err(|e| format!("rigger's project list is not JSON: {e}"))?;
    let projects = projects.as_array().ok_or("rigger's project list is not a list")?;
    let mut repositories = Vec::new();
    let mut skipped = Vec::new();
    for project in projects {
        let field = |key: &str| project.get(key).and_then(|v| v.as_str());
        let Some(name) = field("name") else { continue };
        if field("kind").is_some_and(|kind| kind != "repo") {
            continue;
        }
        if let Some(retired) = field("retired_at") {
            let day = retired.split('T').next().unwrap_or(retired);
            skipped.push(Skipped {
                name: name.to_string(),
                reason: format!("retired in rigger's record on {day}"),
            });
            continue;
        }
        // A weight of 0 leaves the queue, which is what being out of the line is.
        if project.get("weight").and_then(Value::as_u64) == Some(0) {
            skipped.push(Skipped {
                name: name.to_string(),
                reason: "out of the line in rigger's record".to_string(),
            });
            continue;
        }
        let Some(path) = field("path") else {
            skipped.push(Skipped {
                name: name.to_string(),
                reason: "rigger's record has no path for it".to_string(),
            });
            continue;
        };
        repositories.push((name.to_string(), PathBuf::from(path)));
    }
    repositories.sort_by(|a, b| a.0.cmp(&b.0));
    Ok((repositories, skipped))
}

/// The name the record knows the repository at `root` by.
pub fn project_for(root: &Path) -> Result<Option<String>, String> {
    let json = ask(&["--json", "project", "list"])?;
    let projects: Value = serde_json::from_str(&json).map_err(|e| format!("rigger's project list is not JSON: {e}"))?;
    Ok(projects.as_array().into_iter().flatten().find_map(|project| {
        let path = project.get("path")?.as_str()?;
        same_place(Path::new(path), root).then(|| project.get("name")?.as_str().map(str::to_string))?
    }))
}

/// Whether two paths are one directory: by its components once resolved,
/// and on Windows regardless of case, where `C:\Projects` and `c:/projects`
/// are the same place.
fn same_place(a: &Path, b: &Path) -> bool {
    let resolve = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    let (a, b) = (resolve(a), resolve(b));
    let parts = |p: &Path| -> Vec<String> {
        p.components()
            .map(|c| {
                let part = c.as_os_str().to_string_lossy().into_owned();
                if cfg!(windows) { part.to_lowercase() } else { part }
            })
            .collect()
    };
    parts(&a) == parts(&b)
}

/// The stage a project is in, as the record has it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stage {
    pub version: String,
    pub title: String,
    /// Each open task: its id in the record, and what it says.
    pub tasks: Vec<(i64, String)>,
}

/// The stage the record says `project` is building, if it says one.
pub fn stage(project: &str) -> Result<Option<Stage>, String> {
    let json = ask(&["--json", "context", project])?;
    parse_stage(&json)
}

fn parse_stage(json: &str) -> Result<Option<Stage>, String> {
    let context: Value = serde_json::from_str(json).map_err(|e| format!("rigger's context is not JSON: {e}"))?;
    let Some(current) = context.get("current").filter(|c| !c.is_null()) else {
        return Ok(None);
    };
    let text = |key: &str| current.get(key).and_then(Value::as_str).unwrap_or_default().to_string();
    let tasks = current
        .get("tasks")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|task| task.get("status").and_then(Value::as_str) != Some("done"))
        .filter_map(|task| Some((task.get("id")?.as_i64()?, task.get("title")?.as_str()?.to_string())))
        .collect();
    Ok(Some(Stage {
        version: text("version"),
        title: text("title"),
        tasks,
    }))
}

/// Leaves a line in the record about where `project` stands: rigger adds it
/// to the sitting that is open, so the ritual is in the diary next to the
/// work it started.
pub fn note_state(project: &str, text: &str) -> Result<(), String> {
    ask(&["note", project, "--kind", "state", text]).map(|_| ())
}

/// Where a state line ended up besides the hub's state block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Diary {
    /// In the diary of the sitting open since this moment.
    Sitting(String),
    /// No sitting is open: the line is in the state block only.
    NoSitting,
    /// A sitting is open, and this rigger keeps state lines apart from it:
    /// older than 0.47.0.
    Apart,
}

/// Whether `line`, just noted, is in the diary of the sitting open on
/// `project` - which is the point of noting it at the start of a stage.
pub fn diary_of(project: &str, line: &str) -> Result<Diary, String> {
    match ask(&["--json", "session", "draft", project]) {
        Ok(json) => parse_diary(&json, line),
        Err(reason) if reason.contains("no session is open") => Ok(Diary::NoSitting),
        Err(reason) => Err(reason),
    }
}

fn parse_diary(json: &str, line: &str) -> Result<Diary, String> {
    let draft: Value = serde_json::from_str(json).map_err(|e| format!("rigger's draft is not JSON: {e}"))?;
    let session = draft.get("session").ok_or("rigger's draft names no session")?;
    let holds = session
        .get("state")
        .and_then(Value::as_array)
        .is_some_and(|lines| lines.iter().any(|l| l.as_str() == Some(line)));
    if !holds {
        return Ok(Diary::Apart);
    }
    let since = session.get("started_at").and_then(Value::as_str).unwrap_or_default().to_string();
    Ok(Diary::Sitting(since))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_record_s_repositories_are_taken_and_the_rest_passed_over() {
        let json = r#"[
            {"name": "beta", "path": "/src/beta", "kind": "repo", "weight": 1},
            {"name": "alpha", "path": "/src/alpha", "kind": "repo", "weight": 6},
            {"name": "frozen", "path": "/src/frozen", "kind": "repo", "weight": 0},
            {"name": "gone", "path": "/src/gone", "kind": "repo", "weight": 0, "retired_at": "2026-10-13T17:41:26Z"},
            {"name": "line", "path": "service:line", "kind": "service", "weight": 1}
        ]"#;
        let (repositories, skipped) = parse_line(json).unwrap();
        let names: Vec<&str> = repositories.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, ["alpha", "beta"]);
        assert_eq!(repositories[0].1, PathBuf::from("/src/alpha"));
        assert_eq!(
            skipped,
            [
                Skipped {
                    name: "frozen".into(),
                    reason: "out of the line in rigger's record".into()
                },
                Skipped {
                    name: "gone".into(),
                    reason: "retired in rigger's record on 2026-10-13".into()
                }
            ]
        );
    }

    #[test]
    fn a_list_that_is_not_one_is_refused() {
        assert!(parse_line("{}").is_err());
        assert!(parse_line("not json").is_err());
    }

    #[test]
    fn the_stage_is_the_current_version_with_its_open_tasks() {
        let json = r#"{"project": "demo", "current": {"version": "v1.2.0", "title": "Rituals",
            "tasks": [{"id": 7, "title": "check", "status": "new"}, {"id": 8, "title": "done one", "status": "done"}]}}"#;
        assert_eq!(
            parse_stage(json).unwrap(),
            Some(Stage {
                version: "v1.2.0".into(),
                title: "Rituals".into(),
                tasks: vec![(7, "check".into())],
            })
        );
        assert_eq!(parse_stage(r#"{"project": "demo", "current": null}"#).unwrap(), None);
    }

    /// The line is the sitting's when the draft of the open sitting says it;
    /// a draft without it is a rigger that keeps the two apart.
    #[test]
    fn a_state_line_is_in_the_diary_when_the_open_sitting_says_it() {
        let line = "Stage v1.2.0 began at 71c2514; tree clean.";
        let draft = format!(r#"{{"session": {{"started_at": "2026-11-09T11:09:11Z", "state": ["{line}"]}}, "entry": ""}}"#);
        assert_eq!(parse_diary(&draft, line).unwrap(), Diary::Sitting("2026-11-09T11:09:11Z".into()));
        assert_eq!(parse_diary(&draft, "another line").unwrap(), Diary::Apart);
        let older = r#"{"session": {"started_at": "2026-11-09T11:09:11Z"}, "entry": ""}"#;
        assert_eq!(parse_diary(older, line).unwrap(), Diary::Apart);
        assert!(parse_diary("not json", line).is_err());
    }

    #[test]
    fn a_place_is_its_components() {
        let dir = tempfile::tempdir().unwrap();
        let inner = dir.path().join("repo");
        std::fs::create_dir_all(&inner).unwrap();
        assert!(same_place(&inner, &dir.path().join("repo").join(".")));
        assert!(!same_place(&inner, dir.path()));
        if cfg!(windows) {
            let upper = PathBuf::from(inner.display().to_string().to_uppercase());
            assert!(same_place(&inner, &upper));
        }
    }
}
