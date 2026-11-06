//! `lyrn adopt`: an existing repository brought up to the line's standard.
//!
//! The line has repositories older than lyrn, and repositories started by
//! hand. `adopt` reads what one already is - its form, its name, its
//! description, where it lives - and adds the files of the standard it does
//! not have yet: licence, changelog, editor and git settings, the CI gate and
//! the audit, the security policy and the code of conduct, the issue and pull
//! request templates, the first ADRs, and `lyrn.toml`, which is what makes it a
//! lyrn project that `doctor` and `upgrade` can later read.
//!
//! It never replaces a file. Unlike a form added to a repository, it does not
//! refuse when some are present either: each file of the standard stands on
//! its own, so the ones already there are kept, the rest are added, and the
//! tree shown beforehand says which is which. A file under another common
//! name - `LICENSE.md` - counts as present, and so does `docs/adr/` as a whole
//! when the repository keeps decisions of its own.
//!
//! What it adds has to hold in this repository, not in the one the form would
//! have started. So the files are fitted before they are written: a decision
//! the audit points at gets the next free number where the log is the
//! repository's, the audit reads the workspace's manifest where there is one,
//! and every lockfile the form knows nothing of gets its own audit job. Nothing
//! adopt adds should send a reader to a file that is not there, or leave a
//! lockfile for the doctor to find the moment it is committed.

use std::error::Error;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use crate::cli::{AdoptArgs, IdentityArgs};
use crate::commands::check::site::Site;
use crate::commands::doctor::{Repo, audit};
use crate::commands::init::name_of;
use crate::commands::new::{Wanted, build_context};
use crate::generate::{self, Plan, PlannedFile};
use crate::naming;
use crate::repo::{Facts, already_there, origin_repo};
use crate::template::{self, Kind};
use crate::templates::{self, community, docs};

pub fn run(args: AdoptArgs) -> Result<(), Box<dyn Error>> {
    let root = args.path.clone().unwrap_or_else(|| PathBuf::from("."));
    if !root.is_dir() {
        return Err(format!(
            "`{}` is not a directory - `lyrn adopt` brings an existing repository up to the standard",
            root.display()
        )
        .into());
    }

    let facts = Facts::read(&root)?;
    let (form, detected_host) = match args.form {
        Some(form) => (form, None),
        None => facts.form(&root)?,
    };
    if !form.adoptable() {
        return Err(format!("a repository is not a `{form}`: a documentation site is added to one with `lyrn new <name> --form docs`").into());
    }

    let name = match args.name.clone().or_else(|| facts.name.clone()) {
        Some(name) => {
            naming::validate_name(&name).map_err(|reason| format!("the project is called `{name}`, which is not a project name: {reason} - pass `--name`"))?;
            name
        }
        None => name_of(&root)?,
    };

    // What the repository already says about itself wins over a default, and
    // anything given on the command line wins over both.
    let identity = IdentityArgs {
        accent: args
            .identity
            .accent
            .clone()
            .or_else(|| facts.accent.clone())
            .or_else(|| naming::product_accent(&name).map(str::to_string)),
        description: args.identity.description.clone().or_else(|| facts.description.clone()),
        author: args.identity.author.clone(),
        repo: args.identity.repo.clone().or_else(|| origin_repo(&root)),
    };
    let host = args.host.as_deref().or(detected_host);
    let wanted = Wanted {
        name: &name,
        form,
        host,
        with: &[],
        identity: &identity,
    };

    // Your template of the form, if you have one, is the standard you adopt
    // to: the same rule `lyrn new` follows.
    let template = template::for_form(form)?;
    if let Some(notice) = template.origin.notice(Some(form)) {
        println!("{notice}");
    }
    let Kind::Lyrn(native) = &template.kind else {
        unreachable!("a form's template is a lyrn template; `for_form` refuses anything else")
    };
    let interactive = !args.assume_yes && std::io::stdin().is_terminal();
    let context = build_context(&wanted, native, &template.origin, interactive)?;
    let standard = generate::plan_with(&templates::standard_files(&native.files), &native.manifest, &context, &[])?;

    let mut missing = Plan::default();
    let mut notes: Vec<(PathBuf, Option<String>)> = Vec::new();
    for file in standard.files {
        match already_there(&root, &file.path) {
            Some(note) => notes.push((file.path.clone(), Some(note))),
            None => {
                notes.push((file.path.clone(), None));
                missing.files.push(file);
            }
        }
    }
    let repo = Repo::read(&root);
    let decision = keep_the_decision_s_number(&mut missing, &root, &repo, &native.files, &context, &native.manifest)?;
    audit_the_root_workspace(&mut missing, &facts);
    audit_every_lockfile(&mut missing, &root, &repo);
    // What fitting added is shown as added, even under a path the log's rule
    // had set aside: the next free number can be the standard's own.
    for file in &missing.files {
        match notes.iter_mut().find(|(path, _)| *path == file.path) {
            Some((_, note)) => *note = None,
            None => notes.push((file.path.clone(), None)),
        }
    }
    let kept = notes.iter().filter(|(_, note)| note.is_some()).count();

    if missing.files.is_empty() {
        println!("Nothing to add: `{}` already carries every file of the `{form}` standard.", root.display());
        return Ok(());
    }

    let tree = generate::draw_tree(notes.iter().map(|(path, note)| (path.as_path(), note.as_deref())));
    let heading = format!(
        "{} to `{}` as a `{form}` project; {kept} of its standard {} already there:",
        plural(missing.files.len()),
        root.display(),
        if kept == 1 { "is" } else { "are" }
    );

    if args.dry_run {
        println!("Would add {heading}\n");
        println!("{}", indent(&tree));
        return Ok(());
    }
    if interactive {
        println!("\nWill add {heading}\n");
        println!("{}\n", indent(&tree));
        if !confirm("Add them?")? {
            println!("Nothing was written.");
            return Ok(());
        }
    }

    generate::write(&missing, &root)?;
    println!("Added {} to `{}`.", plural(missing.files.len()), root.display());

    println!("\nNext:");
    println!("  git status    # what was added, before it is committed");
    println!("  lyrn doctor   # it reads what git tracks, so it sees these once they are committed");
    if let Some(decision) = decision.filter(|d| root.join(d.rsplit_once('/').map_or(".", |(dir, _)| dir)).join("README.md").is_file()) {
        println!("  list {decision} in the index beside it    # the index is the repository's own");
    }
    if Site::read(&repo).is_none() && !docs::FOREIGN_SITES.iter().any(|m| root.join(m).exists()) {
        println!("  lyrn new {name} --form docs    # it has no documentation site");
    }
    Ok(())
}

/// The decision the audit points at, in a repository that keeps a log of its
/// own: `docs/adr/` was left alone as a whole, and audit.yml and deny.toml
/// would otherwise send the reader to a 0002 that is another decision, or none.
///
/// One it already keeps is used under its own number. Otherwise the decision
/// joins the log at the next free one, and that is what the two files name.
/// The path of the decision, when one was added.
fn keep_the_decision_s_number(
    missing: &mut Plan,
    root: &Path,
    repo: &Repo,
    files: &[generate::TemplateFile],
    context: &crate::model::Context,
    manifest: &crate::model::TemplateManifest,
) -> Result<Option<String>, Box<dyn Error>> {
    const STANDARD: &str = "docs/adr/0002-dependabot-is-off.md";
    const NAME: &str = "dependabot-is-off.md";
    if root.join(STANDARD).exists() || missing.files.iter().any(|f| f.path == Path::new(STANDARD)) {
        return Ok(None);
    }
    // Nothing the plan adds names it: both are the repository's already.
    if !missing.files.iter().any(|f| String::from_utf8_lossy(&f.contents).contains(STANDARD)) {
        return Ok(None);
    }
    let Some(dir) = ["docs/adr", "doc/adr"].into_iter().find(|d| repo.files.iter().any(|f| in_the_log(f, d))) else {
        return Ok(None);
    };
    let in_log: Vec<&str> = repo.files.iter().filter(|f| in_the_log(f, dir)).map(String::as_str).collect();
    let (path, added) = match in_log.iter().find(|f| f.ends_with(&format!("-{NAME}"))) {
        Some(found) => (found.to_string(), None),
        None => {
            let next = in_log
                .iter()
                .filter_map(|f| f.rsplit('/').next()?.split('-').next()?.parse::<u32>().ok())
                .max()
                .map_or(1, |n| n + 1);
            let number = format!("{next:04}");
            let path = format!("{dir}/{number}-{NAME}");
            let source = files
                .iter()
                .find(|f| f.path == STANDARD)
                .ok_or("the standard has no decision about Dependabot")?;
            let rendered = generate::plan(std::slice::from_ref(source), manifest, context)?;
            let body = String::from_utf8_lossy(&rendered.files[0].contents).replace("# ADR 0002:", &format!("# ADR {number}:"));
            (path.clone(), Some((path, body)))
        }
    };
    for file in &mut missing.files {
        let text = String::from_utf8_lossy(&file.contents);
        if text.contains(STANDARD) {
            file.contents = text.replace(STANDARD, &path).into_bytes();
        }
    }
    let Some((added, body)) = added else { return Ok(None) };
    missing.files.push(PlannedFile {
        path: PathBuf::from(&added),
        contents: body.into_bytes(),
        executable: false,
    });
    Ok(Some(added))
}

/// A decision's file directly in the log, not its index.
fn in_the_log(file: &str, dir: &str) -> bool {
    file.strip_prefix(dir)
        .and_then(|rest| rest.strip_prefix('/'))
        .is_some_and(|name| name.ends_with(".md") && !name.contains('/') && !name.eq_ignore_ascii_case("README.md"))
}

/// A desktop app inside a workspace has its Cargo.lock at the top, and the
/// crates beside the window are in it. Told `src-tauri/Cargo.toml`, the audit
/// reads the window alone and leaves that lockfile to nobody.
fn audit_the_root_workspace(missing: &mut Plan, facts: &Facts) {
    if !facts.cargo.as_ref().is_some_and(|c| c.contains_key("workspace")) {
        return;
    }
    let told = "      - uses: EmbarkStudios/cargo-deny-action@v2\n        with:\n          manifest-path: src-tauri/Cargo.toml\n";
    if let Some(audit) = missing.files.iter_mut().find(|f| f.path == Path::new(".github/workflows/audit.yml")) {
        let text = String::from_utf8_lossy(&audit.contents).replace(told, "      - uses: EmbarkStudios/cargo-deny-action@v2\n");
        audit.contents = text.into_bytes();
    }
}

/// A job for every pnpm lockfile the audit it is adding would leave out: the
/// form's audit reads where the form keeps its packages, and a repository older
/// than lyrn keeps them where it likes. Only when the audit is being added: one
/// that is already there is the repository's, and the doctor says what it misses.
fn audit_every_lockfile(missing: &mut Plan, root: &Path, repo: &Repo) {
    let workflow = Path::new(".github/workflows/audit.yml");
    let Some(at) = missing.files.iter().position(|f| f.path == workflow) else {
        return;
    };
    let added = String::from_utf8_lossy(&missing.files[at].contents).into_owned();
    let form_reads_the_top = added.contains("name: Packages\n") && !added.contains("working-directory: frontend");
    let mut jobs = String::new();
    let mut site: Option<String> = None;
    for dir in audit::unaudited_pnpm(repo) {
        if dir == "." && form_reads_the_top {
            continue;
        }
        if dir == "docs" || dir.starts_with("docs/") {
            // The first one the tree lists is the site's; one under it is not.
            if site.is_none() {
                site = Some(dir);
                continue;
            }
        }
        jobs.push_str(&community::packages_job(&dir));
        let tool = PathBuf::from(&dir).join("tools/check-licenses.mjs");
        if already_there(root, &tool).is_none() {
            missing.files.push(PlannedFile {
                path: tool,
                contents: include_str!("../templates/community/check-licenses.mjs.tmpl").as_bytes().to_vec(),
                executable: false,
            });
        }
    }
    missing.files[at].contents = format!("{added}{jobs}").into_bytes();
    let site_workflow = Path::new(".github/workflows/docs-audit.yml");
    if let Some(dir) = site
        && already_there(root, site_workflow).is_none()
    {
        missing.files.push(PlannedFile {
            path: site_workflow.to_path_buf(),
            contents: community::site_audit(&dir).into_bytes(),
            executable: false,
        });
    }
}

fn plural(count: usize) -> String {
    if count == 1 { "1 file".to_string() } else { format!("{count} files") }
}

fn indent(text: &str) -> String {
    text.lines().map(|line| format!("  {line}")).collect::<Vec<_>>().join("\n")
}

fn confirm(prompt: &str) -> Result<bool, Box<dyn Error>> {
    use dialoguer::{Confirm, theme::ColorfulTheme};
    Ok(Confirm::with_theme(&ColorfulTheme::default()).with_prompt(prompt).default(true).interact()?)
}
