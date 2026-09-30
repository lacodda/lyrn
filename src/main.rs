mod cli;
mod commands;
mod generate;
mod host;
mod model;
mod naming;
mod render;
mod template;
mod templates;

use std::process::ExitCode;

use clap::Parser;

use crate::model::Form;

fn main() -> ExitCode {
    let cli = cli::Cli::parse();

    let result = match cli.command {
        cli::Command::New(args) => commands::new::run(*args),
        cli::Command::Init(args) => commands::init::run(*args),
        cli::Command::Adopt(args) => commands::adopt::run(*args),
        cli::Command::Forms => {
            list_forms();
            Ok(())
        }
        cli::Command::Template(command) => commands::template::run(command),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn list_forms() {
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
