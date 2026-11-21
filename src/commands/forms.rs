//! `lyrn forms`: the forms a project can take, their add-ons, dowel's blocks,
//! and your own templates - and with `--consumers`, which product of the line
//! each form made.
//!
//! A form no product is made from is a template nobody holds to a real
//! product: its CI builds a scaffold, and the first time anyone finds out
//! what the scaffold lacks is the day a product is started on it. The list
//! says which ones those are, from the record and from each repository's own
//! `lyrn.toml` - the form it was made or adopted as - rather than from a
//! guess at what its files look like.

use std::collections::BTreeMap;
use std::error::Error;
use std::path::Path;

use crate::cli::FormsArgs;
use crate::commands::new::blocks;
use crate::host;
use crate::model::Form;
use crate::repo::Facts;
use crate::template;
use crate::templates::dowel::Catalogue;

pub fn run(args: FormsArgs) -> Result<(), Box<dyn Error>> {
    if args.consumers {
        let (repositories, _) = crate::rigger::line()?;
        print!("{}", consumers(&repositories));
        return Ok(());
    }
    list();
    Ok(())
}

fn list() {
    // Measured, not assumed: a column wide enough for the longest name today
    // is one character too narrow for whatever is added next, and the only
    // sign is a listing whose second column no longer lines up.
    let form_column = Form::ALL.iter().map(|f| f.as_str().len()).max().unwrap_or(0) + 2;
    let addon_column = Form::ALL.iter().flat_map(|f| f.addons()).map(|a| a.as_str().len()).max().unwrap_or(0) + 2;

    for form in Form::ALL {
        println!("{:<form_column$} {}", form.as_str(), form.summary());
        // Add-ons are listed under the form that understands them, because
        // `--with keyring` means nothing without knowing which forms take it.
        for addon in form.addons() {
            println!("  --with {:<addon_column$} {}", addon.as_str(), addon.summary());
        }
        // A form generated against a host is useless without knowing which
        // hosts exist: `--host` has no default worth guessing, and the answer
        // changes as the line's applications grow plugin hosts of their own.
        if form.takes_a_host() {
            for host in host::ALL {
                println!("  --host {:<addon_column$} {}", host.name, host.about);
            }
        }
    }
    // Blocks are dowel's, not any one form's: listed once, for every form
    // whose files draw with dowel.
    let catalogue = Catalogue::builtin();
    let names: Vec<String> = catalogue
        .blocks()
        .map(|b| format!("{}{}", blocks::PREFIX, b.name.trim_end_matches("-screen")))
        .collect();
    println!(
        "\nDowel's screens, for a form that draws with dowel (spa; service --with spa), from dowel-ui {}:\n  --with {}",
        catalogue.version,
        names.join(", ")
    );
    list_local_templates();
}

/// Your templates under ~/.lyrn/templates, each read the way `lyrn new`
/// would read it - one that cannot be used says why here, not on the day it
/// is needed.
fn list_local_templates() {
    let Some(dir) = template::local_templates_dir().filter(|d| d.is_dir()) else {
        return;
    };
    let Ok(entries) = std::fs::read_dir(&dir) else { return };
    let mut names: Vec<String> = entries
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    if names.is_empty() {
        return;
    }
    names.sort();
    let column = names.iter().map(String::len).max().unwrap_or(0) + 2;

    println!("\nYour templates, in {}:", dir.display());
    for name in names {
        let path = dir.join(&name);
        let origin = template::Origin::Local {
            name: name.clone(),
            dir: path.clone(),
        };
        let about = match template::load_dir(&path, origin) {
            Ok(found) => match found.kind {
                template::Kind::Lyrn(native) => {
                    let instead = if name == native.form.as_str() { ", in place of the built-in one" } else { "" };
                    let summary = if native.manifest.description.is_empty() {
                        String::new()
                    } else {
                        format!(": {}", native.manifest.description)
                    };
                    format!("the {} form{instead}{summary}", native.form)
                }
                template::Kind::CargoGenerate(_) => "a cargo-generate template".to_string(),
            },
            Err(e) => format!("cannot be used - {}", e.to_string().lines().next().unwrap_or_default()),
        };
        println!("  {name:<column$} {about}");
    }
}

/// What a repository of the line was made as.
#[derive(Debug, PartialEq, Eq)]
enum Made {
    /// The form its `lyrn.toml` names.
    Form(Form),
    /// No `lyrn.toml`: neither made nor adopted by lyrn.
    Unheld,
    /// A `lyrn.toml` that names no form, or one this lyrn does not know.
    Unreadable(String),
}

fn made(root: &Path) -> Made {
    match Facts::read(root) {
        Ok(facts) if !facts.has_lyrn_toml => Made::Unheld,
        Ok(facts) => match facts.lyrn_form {
            Some(form) => Made::Form(form),
            None => Made::Unreadable("its lyrn.toml names no form".to_string()),
        },
        Err(reason) => Made::Unreadable(reason.to_string()),
    }
}

/// A documentation site is added to a repository rather than being one, so
/// what it was made from is the site under `docs/`.
fn has_a_site(root: &Path) -> bool {
    root.join("docs").join("astro.config.mjs").is_file()
}

/// The report: each form and the products made from it, then the products
/// held to no form, then the forms nobody makes.
fn consumers(repositories: &[(String, std::path::PathBuf)]) -> String {
    let mut by_form: BTreeMap<Form, Vec<&str>> = BTreeMap::new();
    let mut unheld = Vec::new();
    let mut unreadable = Vec::new();
    for (name, root) in repositories {
        match made(root) {
            Made::Form(form) => by_form.entry(form).or_default().push(name),
            Made::Unheld => unheld.push(name.as_str()),
            Made::Unreadable(reason) => unreadable.push(format!("{name} ({reason})")),
        }
        if has_a_site(root) {
            by_form.entry(Form::Docs).or_default().push(name);
        }
    }

    let column = Form::ALL.iter().map(|f| f.as_str().len()).max().unwrap_or(0) + 2;
    let mut out = format!("Which product of the line each form made, from rigger's record of {}:\n\n", repositories.len());
    for form in Form::ALL {
        let made = by_form.get(form).map(|names| names.join(", ")).unwrap_or_else(|| "none".to_string());
        out.push_str(&format!("{:<column$} {made}\n", form.as_str()));
    }
    if !unheld.is_empty() {
        out.push_str(&format!("\nNo lyrn.toml, so made from no form: {}\n", unheld.join(", ")));
    }
    if !unreadable.is_empty() {
        out.push_str(&format!("\nNot read: {}\n", unreadable.join(", ")));
    }
    let nobody: Vec<&str> = Form::ALL.iter().filter(|f| !by_form.contains_key(f)).map(|f| f.as_str()).collect();
    match nobody.len() {
        0 => out.push_str("\nEvery form has a product made from it.\n"),
        1 => out.push_str(&format!(
            "\n1 form no product is made from: {} - a template nobody makes is one only its own CI has built.\n",
            nobody[0]
        )),
        n => out.push_str(&format!(
            "\n{n} forms no product is made from: {} - a template nobody makes is one only its own CI has built.\n",
            nobody.join(", ")
        )),
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repository(files: &[(&str, &str)]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for (path, text) in files {
            let path = dir.path().join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        dir
    }

    #[test]
    fn each_form_names_the_products_made_from_it_and_the_forms_nobody_makes() {
        let kasl = repository(&[("lyrn.toml", "[project]\nname = \"kasl\"\nform = \"cli\"\n"), ("docs/astro.config.mjs", "")]);
        let turnout = repository(&[("lyrn.toml", "[project]\nname = \"turnout\"\nform = \"cli\"\n")]);
        let furca = repository(&[("lyrn.toml", "[project]\nname = \"furca\"\nform = \"desktop\"\n")]);
        let dowel = repository(&[("package.json", "{\"name\": \"dowel-ui\"}")]);
        let odd = repository(&[("lyrn.toml", "[project]\nname = \"odd\"\n")]);
        let repositories = [
            ("dowel".to_string(), dowel.path().to_path_buf()),
            ("furca".to_string(), furca.path().to_path_buf()),
            ("kasl".to_string(), kasl.path().to_path_buf()),
            ("odd".to_string(), odd.path().to_path_buf()),
            ("turnout".to_string(), turnout.path().to_path_buf()),
        ];
        let report = consumers(&repositories);
        // The column is as wide as `tauri-plugin` and two more.
        let row = |form: &str, made: &str| format!("\n{form:<14} {made}\n");
        for line in [
            "from rigger's record of 5:".to_string(),
            row("cli", "kasl, turnout"),
            row("desktop", "furca"),
            row("docs", "kasl"),
            row("spa", "none"),
            "No lyrn.toml, so made from no form: dowel\n".to_string(),
            "Not read: odd (its lyrn.toml names no form)\n".to_string(),
            "6 forms no product is made from: spa, service, workspace, mono, plugin, tauri-plugin".to_string(),
        ] {
            assert!(report.contains(&line), "no `{line}` in:\n{report}");
        }
    }
}
