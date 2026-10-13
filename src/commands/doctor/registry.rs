//! `--all`: the repositories rigger's record knows.
//!
//! Which projects are yours is written down once, in rigger, with where each
//! lives and which are out of the line; walking a directory would be a second
//! answer to that question, and it would pick up every clone and experiment
//! that happens to sit there.

use std::error::Error;
use std::path::PathBuf;
use std::process::Command as Process;

use super::report::Target;

/// A project the record keeps but the doctor leaves alone, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skipped {
    pub name: String,
    pub reason: String,
}

/// The repositories to examine, and the projects passed over.
pub fn line() -> Result<(Vec<Target>, Vec<Skipped>), Box<dyn Error>> {
    let out = Process::new("rigger").args(["--json", "project", "list"]).output().map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            "`--all` reads your projects from rigger's record, and rigger is not installed here - name the repositories instead: `lyrn doctor <path>...`"
                .to_string()
        } else {
            format!("rigger could not be run: {error}")
        }
    })?;
    if !out.status.success() {
        return Err(format!("`rigger --json project list` failed: {}", String::from_utf8_lossy(&out.stderr).trim()).into());
    }
    parse(&String::from_utf8_lossy(&out.stdout))
}

/// What `rigger --json project list` prints, read into targets: a project
/// with a repository is examined, one the record keeps out of the line is
/// passed over by name, and a place the record keeps for itself is not a
/// project anyone holds to a standard.
fn parse(json: &str) -> Result<(Vec<Target>, Vec<Skipped>), Box<dyn Error>> {
    let projects: serde_json::Value = serde_json::from_str(json).map_err(|e| format!("rigger's project list is not JSON: {e}"))?;
    let projects = projects.as_array().ok_or("rigger's project list is not a list")?;
    let mut targets = Vec::new();
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
        if field("tier") == Some("out") {
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
        targets.push(Target::named(name, PathBuf::from(path)));
    }
    targets.sort_by(|a, b| a.name.cmp(&b.name));
    Ok((targets, skipped))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_record_s_repositories_are_examined_and_the_rest_passed_over() {
        let json = r#"[
            {"name": "beta", "path": "/src/beta", "kind": "repo", "tier": null},
            {"name": "alpha", "path": "/src/alpha", "kind": "repo", "tier": "A"},
            {"name": "frozen", "path": "/src/frozen", "kind": "repo", "tier": "out"},
            {"name": "gone", "path": "/src/gone", "kind": "repo", "tier": "out", "retired_at": "2026-10-13T17:41:26Z"},
            {"name": "line", "path": "service:line", "kind": "service", "tier": null}
        ]"#;
        let (targets, skipped) = parse(json).unwrap();
        let names: Vec<&str> = targets.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, ["alpha", "beta"]);
        assert_eq!(targets[0].path, PathBuf::from("/src/alpha"));
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
        assert!(parse("{}").is_err());
        assert!(parse("not json").is_err());
    }
}
