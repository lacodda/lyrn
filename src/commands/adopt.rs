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

use std::error::Error;
use std::io::IsTerminal;
use std::path::PathBuf;

use crate::cli::{AdoptArgs, IdentityArgs};
use crate::commands::init::name_of;
use crate::commands::new::{Wanted, build_context};
use crate::generate::{self, Plan};
use crate::naming;
use crate::repo::{Facts, already_there, has_a_docs_site, origin_repo};
use crate::template::{self, Kind};
use crate::templates;

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
    let kept = notes.len() - missing.files.len();

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
    if !has_a_docs_site(&root) {
        println!("  lyrn new {name} --form docs    # it has no documentation site");
    }
    Ok(())
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
