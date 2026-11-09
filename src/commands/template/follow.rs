//! `lyrn template follow`: a template repository written out from a
//! built-in form, kept up with that form.
//!
//! A template exported from a form is a copy of it at one version of lyrn,
//! and the form moves on - the standard most of all. Following it is the
//! export again over the repository, a commit, its CI, and a tag once the CI
//! passed: the tag is what `lyrn new --template` and `lyrn upgrade` read, and
//! they read only a tag whose CI passed. A lyrn that writes nothing new into
//! the form leaves the repository alone, so a tag of a template always says
//! that something in it changed.

use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command as Process;
use std::time::{Duration, Instant};

use super::{Exported, Over};
use crate::template::{FILES_DIR, github};

/// How often the CI is asked about while it runs.
const POLL: Duration = Duration::from_secs(15);
/// How long a pushed commit may go with no CI on it before that is the answer.
const NO_CI_FOR: Duration = Duration::from_secs(5 * 60);
/// How long the CI may run before lyrn stops waiting for it.
const RUNNING_FOR: Duration = Duration::from_secs(60 * 60);

pub fn run(paths: &[PathBuf], dry_run: bool) -> Result<(), Box<dyn Error>> {
    let mut failed = Vec::new();
    for (i, path) in paths.iter().enumerate() {
        if i > 0 {
            println!();
        }
        if let Err(error) = follow(path, dry_run) {
            eprintln!("`{}`: {error}", path.display());
            failed.push(path.display().to_string());
        }
    }
    if failed.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{} not followed: {}",
            if failed.len() == 1 { "a template was" } else { "templates were" },
            failed.join(", ")
        )
        .into())
    }
}

fn follow(root: &Path, dry_run: bool) -> Result<(), Box<dyn Error>> {
    let Some(earlier) = Exported::read(root)? else {
        return Err("it holds no template.toml: follow a template `lyrn template export` wrote".into());
    };
    if !is_top_level(root) {
        return Err("it is not the top of a git repository: a template is followed by its commits and tags".into());
    }
    require_clean(root)?;
    git(root, &["pull", "--ff-only", "--quiet"]).map_err(|e| format!("could not bring it level with its upstream: {e}"))?;
    git(root, &["fetch", "--tags", "--quiet"]).map_err(|e| format!("could not read its tags: {e}"))?;
    let url = git(root, &["remote", "get-url", "origin"]).map_err(|_| "it has no `origin` to push to")?;
    let slug = github::slug_of(url.trim()).ok_or_else(|| format!("its origin `{}` is not a GitHub repository, whose CI lyrn asks about", url.trim()))?;

    let version = env!("CARGO_PKG_VERSION");
    let form = earlier.form;
    let over = Over::plan(form, root, &slug)?;
    let recorded = earlier.lyrn.clone().unwrap_or_default();
    let standard_now = crate::template::builtin_manifest(form).standard;
    if !material(&over, root, &recorded) {
        println!("{slug}: the {form} form of lyrn {version} writes nothing the template does not have - nothing to follow.");
        return Ok(());
    }

    let standards = if earlier.standard == standard_now {
        format!("on the standard {standard_now}")
    } else {
        format!("the standard {} to {standard_now}", earlier.standard)
    };
    let from = if recorded.is_empty() {
        "an earlier lyrn".to_string()
    } else {
        format!("lyrn {recorded}")
    };
    println!("{slug}: the {form} form, from {from} to {version}, {standards}: {}", over.counts());
    for (word, paths) in [("changed", &over.changed), ("added", &over.added), ("removed", &over.removed)] {
        for path in paths {
            println!("  {:8} {}", word, posix(path));
        }
    }
    if dry_run {
        println!("Nothing was written.");
        return Ok(());
    }

    over.write(root)?;
    git(root, &["add", "--all", "--", "."])?;
    let subject = format!("feat: the {form} form of lyrn {version}");
    let body = format!("Written out again by `lyrn template follow`, from {from} to lyrn {version}: {standards}.");
    git(root, &["commit", "--quiet", "-m", &subject, "-m", &body])?;
    git(root, &["push", "--quiet", "origin", "HEAD"]).map_err(|e| format!("the commit is made here, but the push failed: {e}"))?;
    let commit = git(root, &["rev-parse", "HEAD"])?.trim().to_string();
    println!("Pushed {} to {slug}: {subject}", github::short(&commit));

    wait_for_ci(&slug, &commit)?;
    let tag = next_tag(&git(root, &["tag", "--list"])?);
    let message = format!("{tag}: the {form} form of lyrn {version}");
    git(root, &["tag", "--annotate", &tag, "-m", &message, &commit])?;
    git(root, &["push", "--quiet", "origin", &format!("refs/tags/{tag}")])
        .map_err(|e| format!("the tag {tag} is made here, but the push failed: {e} - `git push origin {tag}` once it can"))?;
    println!("Tagged {slug}@{tag} on {}, whose CI passed:", github::short(&commit));
    println!("  lyrn new my-app --template {slug}@{tag}");
    Ok(())
}

/// Whether the export says anything the repository does not: a file of the
/// project changed, added or gone, or the template's own files changed by
/// more than the version of lyrn they name. Without that last part every
/// release of lyrn would be a release of every template, saying nothing.
fn material(over: &Over, root: &Path, recorded: &str) -> bool {
    if !over.added.is_empty() || !over.removed.is_empty() {
        return true;
    }
    let version = env!("CARGO_PKG_VERSION");
    over.changed.iter().any(|path| {
        if path.starts_with(FILES_DIR) || recorded.is_empty() {
            return true;
        }
        let Some(planned) = over.plan.files.iter().find(|f| &f.path == path) else {
            return true;
        };
        let as_then = String::from_utf8_lossy(&planned.contents).replace(version, recorded);
        std::fs::read_to_string(root.join(path)).map_or(true, |there| there != as_then)
    })
}

/// Wait until the CI of `commit` has finished, and say whether it passed.
fn wait_for_ci(slug: &str, commit: &str) -> Result<(), Box<dyn Error>> {
    let started = Instant::now();
    let mut said = false;
    loop {
        match github::ci(slug, commit)? {
            github::Verdict::Green => {
                println!("The CI of {} passed.", github::short(commit));
                return Ok(());
            }
            github::Verdict::Failed(names) => {
                return Err(format!(
                    "the CI of {slug} at {} failed: {}; nothing was tagged - mend the form in lyrn, and follow again with the lyrn that has it",
                    github::short(commit),
                    names.join(", ")
                )
                .into());
            }
            github::Verdict::NoCi if started.elapsed() > NO_CI_FOR => {
                return Err(format!(
                    "no CI ran on {} in five minutes; nothing was tagged - a template is used only at a tag whose CI passed",
                    github::short(commit)
                )
                .into());
            }
            github::Verdict::Running(names) if started.elapsed() > RUNNING_FOR => {
                return Err(format!(
                    "the CI of {} has run for an hour ({}); nothing was tagged - once it passes, `git tag -a` the commit and push the tag",
                    github::short(commit),
                    names.join(", ")
                )
                .into());
            }
            _ => {
                if !said {
                    println!("Waiting for the CI of {}...", github::short(commit));
                    said = true;
                }
                std::thread::sleep(POLL);
            }
        }
    }
}

/// The tag after the newest one: its minor version up, as a template that
/// follows a form changes what it writes without breaking how it is used.
/// The first is `v1.0.0`.
fn next_tag(listing: &str) -> String {
    let newest = listing
        .lines()
        .map(str::trim)
        .filter(|t| {
            t.strip_prefix('v')
                .is_some_and(|rest| rest.split('.').count() == 3 && rest.split('.').all(|p| p.parse::<u64>().is_ok()))
        })
        .max_by(|a, b| crate::registries::version_cmp(a, b));
    match newest {
        None => "v1.0.0".to_string(),
        Some(tag) => {
            let parts: Vec<u64> = tag[1..].split('.').map(|p| p.parse().unwrap_or(0)).collect();
            format!("v{}.{}.0", parts[0], parts[1] + 1)
        }
    }
}

/// Refuse a work tree with changes not yet committed: an export written over
/// them would mix the two, and nobody could tell afterwards which was which.
pub(crate) fn require_clean(root: &Path) -> Result<(), Box<dyn Error>> {
    if git(root, &["rev-parse", "--is-inside-work-tree"]).is_ok_and(|out| out.trim() == "true") {
        let status = git(root, &["status", "--porcelain"])?;
        if !status.trim().is_empty() {
            let files: Vec<&str> = status.lines().map(|l| l.get(3..).unwrap_or(l)).collect();
            return Err(format!(
                "nothing was written: it has changes not yet committed, which the export would write over - commit or stash them first:\n  {}",
                files.join("\n  ")
            )
            .into());
        }
    }
    Ok(())
}

fn is_top_level(root: &Path) -> bool {
    let Ok(top) = git(root, &["rev-parse", "--show-toplevel"]) else {
        return false;
    };
    let resolve = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    resolve(Path::new(top.trim())) == resolve(root)
}

fn git(root: &Path, args: &[&str]) -> Result<String, String> {
    let out = Process::new("git")
        .args(args)
        .current_dir(root)
        // A push nobody is there to give a password to would hang the run.
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .map_err(|e| format!("cannot run git: {e}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

fn posix(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_next_tag_is_the_newest_one_s_minor_up() {
        assert_eq!(next_tag(""), "v1.0.0");
        assert_eq!(next_tag("v1.0.0\nv1.0.1\nv1.1.0\n"), "v1.2.0");
        // By version, not by the order git lists them in.
        assert_eq!(next_tag("v1.10.0\nv1.9.0\nv1.2.3\n"), "v1.11.0");
        // A tag that is not a version is no one's predecessor.
        assert_eq!(next_tag("latest\nv2.0.0-rc.1\nv1.1.0\n"), "v1.2.0");
    }
}
