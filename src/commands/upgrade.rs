//! `lyrn upgrade`: a repository caught up with the newest version of the
//! line's standard.
//!
//! The standard moves on - a workflow gains a step, `cliff.toml` a group - and
//! the repositories it was written into stay where they were. `lyrn.toml` says
//! which version each one has; `upgrade` carries it to the version this lyrn
//! writes, file by file, as a three-way merge: the standard as it was written
//! then, kept in [`crate::standard`]'s archive, the file as the repository has
//! it now, and the standard as it is written today - the first and the last
//! rendered with the repository's own name, description and accent.
//!
//! What the repository changed in a file is kept, and what the standard
//! changed is brought in. Where both changed the same lines the file gets the
//! markers git uses, and the run ends non-zero until a person has decided. The
//! README is never merged: it is the product's words, and the standard's
//! README was only ever their first draft.
//!
//! A clean upgrade is committed as `chore(standard): ...`. That commit is the
//! changelog's line saying which version the repository caught up with: the
//! changelog is git-cliff's, written from the commits at release time, and a
//! line typed into it by hand would land under the wrong release, or be
//! written over.

use std::error::Error;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::Command as Process;

use crate::cli::{IdentityArgs, UpgradeArgs};
use crate::commands::new::{Wanted, build_context};
use crate::generate::{self, Plan};
use crate::host;
use crate::model::Form;
use crate::repo::{Facts, already_there, origin_repo};
use crate::standard::{self, Revision};
use crate::template::{self, Kind, Origin};
use crate::templates;

/// The files of the standard an upgrade leaves to the repository.
const NEVER_MERGED: &[&str] = &["README.md"];

pub fn run(args: UpgradeArgs) -> Result<bool, Box<dyn Error>> {
    let root = args.path.clone().unwrap_or_else(|| PathBuf::from("."));
    let facts = Facts::read(&root)?;
    if !facts.has_lyrn_toml {
        return Err(format!(
            "`{}` has no lyrn.toml, so nothing says which standard it is on - `lyrn adopt` makes it a lyrn project first",
            root.display()
        )
        .into());
    }
    let stamp = Stamp::read(&root)?;
    if stamp.source != "lyrn" {
        return Err(format!(
            "its files came from the template `{}`, and `lyrn upgrade` follows the standard of lyrn's own forms",
            stamp.source
        )
        .into());
    }

    let to = standard::current();
    match standard::order(&stamp.standard, &to) {
        std::cmp::Ordering::Equal => {
            println!("`{}` is on the standard {to}, the newest this lyrn writes: nothing to do.", root.display());
            return Ok(true);
        }
        std::cmp::Ordering::Greater => {
            return Err(format!(
                "lyrn.toml says the standard {}, newer than {to}, which this lyrn writes - update lyrn first",
                stamp.standard
            )
            .into());
        }
        std::cmp::Ordering::Less => {}
    }
    let Some(from) = standard::find(&stamp.standard) else {
        let oldest = standard::revisions().first().map(|r| r.version.clone()).unwrap_or_default();
        return Err(format!(
            "lyrn.toml says the standard {}, which this lyrn keeps no record of - the oldest it upgrades from is {oldest}",
            stamp.standard
        )
        .into());
    };

    let (old, new) = render(&root, &facts, &stamp, &from)?;
    let steps = plan(&root, &old, &new, &from.version, &to)?;
    let crossed = standard::between(&from.version, &to);

    println!(
        "lyrn upgrade: `{}`, a {} project, from the standard {} to {to}\n",
        root.display(),
        stamp.form,
        from.version
    );
    println!("What the standard changed since:");
    for revision in &crossed {
        println!("  {}", revision.version);
        for change in &revision.changes {
            println!("    - {change}");
        }
    }
    println!();

    let touched: Vec<&Step> = steps.iter().filter(|s| s.writes()).collect();
    print!("{}", summary(&steps));

    let in_git = inside_a_work_tree(&root);
    if in_git {
        let dirty = uncommitted(&root, &touched)?;
        if !dirty.is_empty() {
            return Err(format!(
                "nothing was written: the upgrade changes files with changes of your own not yet committed - commit or stash them first:\n  {}",
                dirty.join("\n  ")
            )
            .into());
        }
    }

    let interactive = !args.assume_yes && std::io::stdin().is_terminal();
    if args.dry_run || interactive {
        for step in &touched {
            print!("\n{}", step.diff());
        }
    }
    if args.dry_run {
        println!("\nNothing was written.");
        return Ok(true);
    }
    if interactive && !touched.is_empty() && !confirm("Write them?")? {
        println!("Nothing was written.");
        return Ok(true);
    }

    for step in &touched {
        step.write(&root)?;
    }

    let conflicted: Vec<&&Step> = touched.iter().filter(|s| matches!(s.action, Action::Conflict(_))).collect();
    let subject = format!("chore(standard): take the line's standard {to}");
    let paths: Vec<String> = touched.iter().map(|s| posix(&s.path)).collect();
    if !conflicted.is_empty() {
        let places: usize = conflicted.iter().map(|s| if let Action::Conflict(n) = s.action { n } else { 0 }).sum();
        println!(
            "\n{} {} a hand: {places} {} marked `<<<<<<< yours` ... `>>>>>>> standard {to}`, where the repository and the standard both changed the same lines.",
            conflicted.len(),
            if conflicted.len() == 1 { "file needs" } else { "files need" },
            if places == 1 { "place is" } else { "places are" },
        );
        println!("Resolve them, then commit the upgrade:");
        println!("  git add {}", paths.join(" "));
        println!("  git commit -m \"{subject}\"");
        return Ok(false);
    }
    if !in_git {
        println!(
            "\nWrote {}; `{}` is not a git repository, so nothing was committed.",
            plural(touched.len()),
            root.display()
        );
        return Ok(true);
    }
    let body = message_body(&from.version, &crossed);
    commit(&root, &paths, &subject, &body)?;
    println!("\nCommitted: {subject}");
    println!("  git show --stat    # what the upgrade changed");
    Ok(true)
}

/// What `lyrn.toml` says the repository is, and which standard it carries.
struct Stamp {
    name: String,
    form: Form,
    source: String,
    revision: String,
    version: String,
    standard: String,
    with: Vec<String>,
}

impl Stamp {
    fn read(root: &Path) -> Result<Self, Box<dyn Error>> {
        let text = std::fs::read_to_string(root.join("lyrn.toml"))?;
        let table: toml::Table = toml::from_str(&text).map_err(|e| format!("lyrn.toml does not parse: {e}"))?;
        let field = |section: &str, key: &str| table.get(section).and_then(|s| s.get(key)).and_then(|v| v.as_str()).map(str::to_string);
        let name = field("project", "name").ok_or("lyrn.toml has no `[project] name`")?;
        let form = field("project", "form")
            .ok_or("lyrn.toml has no `[project] form`")?
            .parse()
            .map_err(|e| format!("lyrn.toml: {e}"))?;
        let standard = field("template", "standard").ok_or("lyrn.toml has no `[template] standard`, so nothing says which standard it is on")?;
        let revision = field("template", "revision").unwrap_or_default();
        Ok(Self {
            name,
            form,
            source: field("template", "source").unwrap_or_else(|| "lyrn".to_string()),
            version: field("template", "version").unwrap_or_else(|| revision.clone()),
            revision,
            standard,
            // A repository adopted, or generated before lyrn recorded them,
            // has no add-ons: adopt writes the standard without any.
            with: table
                .get("template")
                .and_then(|t| t.get("with"))
                .and_then(|w| w.as_array())
                .map(|items| items.iter().filter_map(|i| i.as_str().map(str::to_string)).collect())
                .unwrap_or_default(),
        })
    }
}

/// The standard's files as the repository's version wrote them, and as this
/// lyrn writes them, both rendered with what the repository says it is.
fn render(root: &Path, facts: &Facts, stamp: &Stamp, from: &Revision) -> Result<(Plan, Plan), Box<dyn Error>> {
    let template = template::builtin(stamp.form);
    let Kind::Lyrn(native) = &template.kind else {
        unreachable!("a built-in form is a lyrn template")
    };
    let Some(then) = from.files(stamp.form) else {
        return Err(format!("the standard {} had no `{}` form", from.version, stamp.form).into());
    };

    // A plugin is named after its host, which is how its host finds it.
    let host = match stamp.form {
        Form::Plugin => Some(
            host::ALL
                .iter()
                .find(|h| stamp.name.starts_with(&h.prefix()))
                .map(|h| h.name)
                .ok_or_else(|| format!("`{}` is a plugin, and its name starts with the prefix of no host lyrn knows", stamp.name))?,
        ),
        _ => None,
    };
    let identity = IdentityArgs {
        accent: facts.accent.clone(),
        description: facts.description.clone(),
        author: None,
        repo: origin_repo(root),
    };
    let wanted = Wanted {
        name: &stamp.name,
        form: stamp.form,
        host,
        with: &stamp.with,
        identity: &identity,
    };
    let now = build_context(&wanted, native, &Origin::BuiltIn, false)?;
    // The version the repository is on is rendered with what it was written
    // with, so the lines that say the version are lines the upgrade changes.
    let mut was = now.clone();
    was.set("standard", &from.version)
        .set("template_source", &stamp.source)
        .set("template_revision", &stamp.revision)
        .set("lyrn_version", &stamp.version);

    let old = generate::plan_with(&then, &from.manifest(), &was, &stamp.with)?;
    let new = generate::plan_with(&templates::standard_files(&native.files), &native.manifest, &now, &stamp.with)?;
    Ok((old, new))
}

/// What happens to one file.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Action {
    /// The repository did not have it.
    Add,
    /// The standard's change, into a file the repository had not changed.
    Update,
    /// The standard's change, beside the repository's own.
    Merge,
    /// The repository and the standard changed the same lines, in this many
    /// places: marked for a person.
    Conflict(usize),
    /// The standard no longer writes it, and the repository had not changed it.
    Remove,
    /// Left as it is, and why.
    Keep(String),
}

#[derive(Debug)]
struct Step {
    path: PathBuf,
    action: Action,
    /// The file as the repository has it, with `\n` line ends.
    before: String,
    /// The file as the upgrade leaves it; `None` when it goes.
    after: Option<String>,
    /// Whether the repository keeps it with `\r\n`, as it is written back.
    crlf: bool,
}

impl Step {
    fn writes(&self) -> bool {
        !matches!(self.action, Action::Keep(_))
    }

    fn diff(&self) -> String {
        let name = posix(&self.path);
        let before = if self.action == Action::Add { String::new() } else { self.before.clone() };
        let after = self.after.clone().unwrap_or_default();
        let mut options = diffy::DiffOptions::new();
        options.set_original_filename(format!("a/{name}")).set_modified_filename(format!("b/{name}"));
        options.create_patch(&before, &after).to_string()
    }

    fn write(&self, root: &Path) -> Result<(), Box<dyn Error>> {
        let target = root.join(&self.path);
        match &self.after {
            None => std::fs::remove_file(&target)?,
            Some(text) => {
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let text = if self.crlf { text.replace('\n', "\r\n") } else { text.clone() };
                std::fs::write(&target, text)?;
            }
        }
        Ok(())
    }
}

/// Each file of either version of the standard, against the repository.
fn plan(root: &Path, old: &Plan, new: &Plan, from: &str, to: &str) -> Result<Vec<Step>, Box<dyn Error>> {
    let text = |plan: &Plan, path: &Path| {
        plan.files
            .iter()
            .find(|f| f.path == path)
            .map(|f| String::from_utf8_lossy(&f.contents).into_owned())
    };
    let mut paths: Vec<PathBuf> = old.files.iter().chain(&new.files).map(|f| f.path.clone()).collect();
    paths.sort();
    paths.dedup();

    let mut steps = Vec::new();
    for path in paths {
        if NEVER_MERGED.iter().any(|p| Path::new(p) == path) {
            continue;
        }
        let ours = match std::fs::read(root.join(&path)) {
            Ok(bytes) => Some(String::from_utf8(bytes).map_err(|_| format!("`{}` is not text", posix(&path)))?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(format!("cannot read `{}`: {e}", posix(&path)).into()),
        };
        let crlf = ours.as_deref().is_some_and(|t| t.contains("\r\n"));
        let ours = ours.map(|t| t.replace("\r\n", "\n"));
        let step = |action, before: &str, after: Option<String>| Step {
            path: path.clone(),
            action,
            before: before.to_string(),
            after,
            crlf,
        };

        let (base, theirs) = (text(old, &path), text(new, &path));
        match (base, ours, theirs) {
            (_, None, None) => {}
            (_, None, Some(theirs)) => match already_there(root, &path) {
                Some(note) => steps.push(step(Action::Keep(note), "", None)),
                None => steps.push(step(Action::Add, "", Some(theirs))),
            },
            (Some(base), Some(ours), None) => {
                if ours == base {
                    steps.push(step(Action::Remove, &ours, None));
                } else {
                    steps.push(step(
                        Action::Keep("the standard no longer writes it, and the repository changed it: it is the repository's own now".to_string()),
                        &ours,
                        None,
                    ));
                }
            }
            (None, Some(_), None) => {}
            (base, Some(ours), Some(theirs)) => {
                if base.as_deref() == Some(theirs.as_str()) {
                    // The standard did not change it.
                    continue;
                }
                let base = base.unwrap_or_default();
                let merged = merge(&base, &ours, &theirs, from, to);
                if merged.conflicts > 0 {
                    steps.push(step(Action::Conflict(merged.conflicts), &ours, Some(merged.text)));
                } else if merged.text == ours {
                    // The repository already says it.
                } else if ours == base {
                    steps.push(step(Action::Update, &ours, Some(merged.text)));
                } else {
                    steps.push(step(Action::Merge, &ours, Some(merged.text)));
                }
            }
        }
    }
    Ok(steps)
}

/// A three-way merge, and how many places in it were left for a person.
struct Merged {
    text: String,
    conflicts: usize,
}

/// Merge the standard's change into the repository's file.
///
/// The markers name the three sides by what they are: the repository's
/// version, and the two versions of the standard.
fn merge(base: &str, ours: &str, theirs: &str, from: &str, to: &str) -> Merged {
    let mut options = diffy::MergeOptions::new();
    options
        .set_conflict_style(diffy::ConflictStyle::Diff3)
        .set_incomplete_hunk_style(diffy::IncompleteHunkStyle::Git);
    match options.merge(base, ours, theirs) {
        Ok(text) => Merged { text, conflicts: 0 },
        Err(text) => {
            let mut conflicts = 0;
            let relabelled: String = text
                .split_inclusive('\n')
                .map(|line| match line.trim_end_matches('\n') {
                    "<<<<<<< ours" => {
                        conflicts += 1;
                        "<<<<<<< yours\n".to_string()
                    }
                    "||||||| original" => format!("||||||| standard {from}\n"),
                    ">>>>>>> theirs" => format!(">>>>>>> standard {to}\n"),
                    _ => line.to_string(),
                })
                .collect();
            Merged { text: relabelled, conflicts }
        }
    }
}

fn summary(steps: &[Step]) -> String {
    let width = steps.iter().map(|s| posix(&s.path).chars().count()).max().unwrap_or(0);
    let mut out = String::new();
    for step in steps {
        let what = match &step.action {
            Action::Add => "added".to_string(),
            Action::Update => "updated".to_string(),
            Action::Merge => "merged: the standard's change, beside the repository's own".to_string(),
            Action::Conflict(1) => "needs a hand: both changed the same lines in 1 place".to_string(),
            Action::Conflict(n) => format!("needs a hand: both changed the same lines in {n} places"),
            Action::Remove => "removed: the standard no longer writes it".to_string(),
            Action::Keep(why) => format!("kept: {why}"),
        };
        out.push_str(&format!("  {:width$}  {what}\n", posix(&step.path)));
    }
    out
}

fn message_body(from: &str, crossed: &[Revision]) -> String {
    let mut body = format!("Brought from the standard {from} by lyrn upgrade.\n");
    for revision in crossed {
        body.push_str(&format!("\n{}:\n", revision.version));
        for change in &revision.changes {
            body.push_str(&format!("- {change}\n"));
        }
    }
    body
}

fn inside_a_work_tree(root: &Path) -> bool {
    git(root, &["rev-parse", "--is-inside-work-tree"]).is_ok_and(|out| out.trim() == "true")
}

/// The files among those the upgrade writes that differ from the last
/// commit: the merge reads them as the repository's version, and a commit of
/// the upgrade would carry the person's changes under the standard's name.
fn uncommitted(root: &Path, steps: &[&Step]) -> Result<Vec<String>, Box<dyn Error>> {
    if steps.is_empty() {
        return Ok(Vec::new());
    }
    let mut args = vec!["status".to_string(), "--porcelain".to_string(), "--".to_string()];
    args.extend(steps.iter().map(|s| posix(&s.path)));
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let out = git(root, &args)?;
    Ok(out.lines().map(|line| line.get(3..).unwrap_or(line).to_string()).collect())
}

fn commit(root: &Path, paths: &[String], subject: &str, body: &str) -> Result<(), Box<dyn Error>> {
    let mut add = vec!["add", "--"];
    add.extend(paths.iter().map(String::as_str));
    git(root, &add)?;
    // Only the upgrade's files: whatever else is staged stays staged.
    let mut commit = vec!["commit", "--quiet", "-m", subject, "-m", body, "--"];
    commit.extend(paths.iter().map(String::as_str));
    git(root, &commit).map_err(|e| format!("the files are written, but the commit failed: {e}"))?;
    Ok(())
}

fn git(root: &Path, args: &[&str]) -> Result<String, String> {
    let out = Process::new("git")
        .args(args)
        .current_dir(root)
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

fn plural(count: usize) -> String {
    if count == 1 { "1 file".to_string() } else { format!("{count} files") }
}

fn confirm(prompt: &str) -> Result<bool, Box<dyn Error>> {
    use dialoguer::{Confirm, theme::ColorfulTheme};
    Ok(Confirm::with_theme(&ColorfulTheme::default()).with_prompt(prompt).default(true).interact()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::PlannedFile;

    const OLDEST: &str = "2026.10";

    /// A repository of `form` as the oldest standard in the archive wrote it,
    /// and both versions of the standard rendered for it.
    fn on_the_oldest(form: Form) -> (tempfile::TempDir, Plan, Plan) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let name = if form == Form::Plugin { "kilna-plugin-demo" } else { "demo-app" };
        for args in [
            vec!["init", "--quiet"],
            vec!["remote", "add", "origin", &format!("https://github.com/acme/{name}.git")],
        ] {
            assert!(Process::new("git").args(&args).current_dir(root).status().unwrap().success());
        }
        let stamp = Stamp {
            name: name.to_string(),
            form,
            source: "lyrn".to_string(),
            revision: "2.10.0".to_string(),
            version: "2.10.0".to_string(),
            standard: OLDEST.to_string(),
            with: Vec::new(),
        };
        let facts = Facts {
            accent: Some("#6D7BF2".to_string()),
            description: Some("A demo of the line's standard.".to_string()),
            ..Facts::default()
        };
        let (old, new) = render(root, &facts, &stamp, &standard::find(OLDEST).unwrap()).unwrap();
        // The old version is rendered as it was written: with the version of
        // the standard and of lyrn the repository records, not today's.
        let stamp_then = text(old.files.iter().find(|f| f.path == Path::new("lyrn.toml")).unwrap());
        assert!(
            stamp_then.contains(&format!("standard = \"{OLDEST}\"")) && stamp_then.contains("version = \"2.10.0\""),
            "{stamp_then}"
        );
        generate::write(&old, root).unwrap();
        (dir, old, new)
    }

    fn read(root: &Path, path: &Path) -> Option<String> {
        std::fs::read_to_string(root.join(path)).ok()
    }

    fn text(file: &PlannedFile) -> String {
        String::from_utf8(file.contents.clone()).unwrap()
    }

    /// The whole promise in one: a repository that kept the standard's files
    /// as they were written comes out holding exactly what this lyrn writes.
    #[test]
    fn an_untouched_repository_becomes_what_the_standard_writes_now() {
        let to = standard::current();
        for form in Form::ALL.iter().copied().filter(|f| f.adoptable()) {
            let (dir, old, new) = on_the_oldest(form);
            let root = dir.path();
            let steps = plan(root, &old, &new, OLDEST, &to).unwrap();
            // The standard changed the README between the two versions; it is
            // the product's all the same.
            assert!(steps.iter().all(|s| s.path != Path::new("README.md")), "{form}: the README was merged");
            for step in &steps {
                assert!(
                    matches!(step.action, Action::Add | Action::Update | Action::Remove),
                    "{form}: `{}` is {:?}, in a repository that changed nothing",
                    posix(&step.path),
                    step.action
                );
                step.write(root).unwrap();
            }
            for file in &new.files {
                if NEVER_MERGED.iter().any(|p| Path::new(p) == file.path) {
                    continue;
                }
                assert_eq!(read(root, &file.path), Some(text(file)), "{form}: `{}`", posix(&file.path));
            }
            for file in old.files.iter().filter(|f| !new.files.iter().any(|n| n.path == f.path)) {
                assert!(!root.join(&file.path).exists(), "{form}: `{}` outlived the standard", posix(&file.path));
            }
            assert!(
                read(root, Path::new("lyrn.toml")).unwrap().contains(&format!("standard = \"{to}\"")),
                "{form}: lyrn.toml does not say the standard it now carries"
            );
        }
    }

    /// The repository's own line, on top of each file the standard changes,
    /// is still there afterwards, with the standard's change below it.
    #[test]
    fn what_the_repository_added_is_kept() {
        let to = standard::current();
        let (dir, old, new) = on_the_oldest(Form::Cli);
        let root = dir.path();
        let changed: Vec<PathBuf> = plan(root, &old, &new, OLDEST, &to)
            .unwrap()
            .into_iter()
            .filter(|s| s.action == Action::Update)
            .map(|s| s.path)
            .collect();
        assert!(!changed.is_empty(), "the standard changed nothing the cli form writes");
        let own = |path: &Path| {
            if path.extension().is_some_and(|e| e == "md") {
                "<!-- the repository's own -->\n"
            } else {
                "# the repository's own\n"
            }
        };
        for path in &changed {
            let before = read(root, path).unwrap();
            std::fs::write(root.join(path), format!("{}{before}", own(path))).unwrap();
        }
        for step in plan(root, &old, &new, OLDEST, &to).unwrap().into_iter().filter(|s| changed.contains(&s.path)) {
            assert_eq!(step.action, Action::Merge, "`{}`", posix(&step.path));
            let theirs = text(new.files.iter().find(|f| f.path == step.path).unwrap());
            assert_eq!(
                step.after.as_deref(),
                Some(format!("{}{theirs}", own(&step.path)).as_str()),
                "`{}`",
                posix(&step.path)
            );
        }
    }

    /// Where the repository changed the very lines the standard changed, the
    /// file is marked for a person and the step says how many places.
    #[test]
    fn lines_both_changed_are_left_to_a_person() {
        let to = standard::current();
        let (dir, old, new) = on_the_oldest(Form::Cli);
        let root = dir.path();
        let path = plan(root, &old, &new, OLDEST, &to)
            .unwrap()
            .into_iter()
            .find(|s| s.action == Action::Update)
            .map(|s| s.path)
            .unwrap();
        let base = text(old.files.iter().find(|f| f.path == path).unwrap());
        let theirs = text(new.files.iter().find(|f| f.path == path).unwrap());
        let at = base.lines().zip(theirs.lines()).position(|(b, t)| b != t).unwrap();
        let ours: Vec<String> = base
            .lines()
            .enumerate()
            .map(|(i, l)| if i == at { "the repository's own".to_string() } else { l.to_string() })
            .collect();
        std::fs::write(root.join(&path), ours.join("\n") + "\n").unwrap();

        let step = plan(root, &old, &new, OLDEST, &to).unwrap().into_iter().find(|s| s.path == path).unwrap();
        assert!(matches!(step.action, Action::Conflict(n) if n >= 1), "{:?}", step.action);
        let after = step.after.unwrap();
        for marker in ["<<<<<<< yours", &format!("||||||| standard {OLDEST}"), &format!(">>>>>>> standard {to}")] {
            assert!(after.lines().any(|l| l == marker), "no `{marker}` in:\n{after}");
        }
        assert!(after.contains("the repository's own"));
    }

    fn planned(files: &[(&str, &str)]) -> Plan {
        Plan {
            files: files
                .iter()
                .map(|(path, text)| PlannedFile {
                    path: PathBuf::from(path),
                    contents: text.as_bytes().to_vec(),
                    executable: false,
                })
                .collect(),
        }
    }

    #[test]
    fn a_file_the_standard_dropped_goes_only_if_the_repository_left_it_alone() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let old = planned(&[("gone.txt", "as written\n"), ("kept.txt", "as written\n")]);
        let new = planned(&[]);
        std::fs::write(root.join("gone.txt"), "as written\n").unwrap();
        std::fs::write(root.join("kept.txt"), "changed here\n").unwrap();
        let steps = plan(root, &old, &new, "a", "b").unwrap();
        assert_eq!(steps[0].action, Action::Remove);
        assert!(matches!(&steps[1].action, Action::Keep(why) if why.contains("repository's own")));
    }

    /// A checkout with CRLF line ends is merged on its lines, not as one
    /// changed line per line, and written back as it was kept.
    #[test]
    fn a_crlf_checkout_is_merged_and_written_back_with_crlf() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join("f.yml"), "one\r\ntwo\r\n").unwrap();
        let steps = plan(root, &planned(&[("f.yml", "one\ntwo\n")]), &planned(&[("f.yml", "one\n2\n")]), "a", "b").unwrap();
        assert_eq!(steps[0].action, Action::Update);
        steps[0].write(root).unwrap();
        assert_eq!(std::fs::read_to_string(root.join("f.yml")).unwrap(), "one\r\n2\r\n");
    }

    #[test]
    fn a_licence_kept_under_another_name_is_left_alone() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join("LICENSE.md"), "MIT\n").unwrap();
        let steps = plan(root, &planned(&[("LICENSE", "MIT\n")]), &planned(&[("LICENSE", "MIT, 2026\n")]), "a", "b").unwrap();
        assert!(matches!(&steps[0].action, Action::Keep(why) if why.contains("LICENSE.md")));
    }

    #[test]
    fn a_change_of_the_standard_lands_beside_the_repository_s_own() {
        let base = "one\ntwo\nthree\n";
        let ours = "one\ntwo\nthree\nfour, the repository's\n";
        let theirs = "one\n2, the standard's\nthree\n";
        let merged = merge(base, ours, theirs, "2026.10", "2026.11");
        assert_eq!(merged.conflicts, 0);
        assert_eq!(merged.text, "one\n2, the standard's\nthree\nfour, the repository's\n");
    }

    #[test]
    fn the_same_lines_changed_twice_are_marked_with_the_versions() {
        let merged = merge("a\nb\nc\n", "a\nmine\nc\n", "a\nstandard\nc\n", "2026.10", "2026.11");
        assert_eq!(merged.conflicts, 1);
        assert_eq!(
            merged.text,
            "a\n<<<<<<< yours\nmine\n||||||| standard 2026.10\nb\n=======\nstandard\n>>>>>>> standard 2026.11\nc\n"
        );
    }
}
