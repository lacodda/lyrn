mod cli;
mod commands;
mod generate;
mod model;
mod naming;
mod render;
mod templates;

use std::process::ExitCode;

use clap::Parser;

use crate::model::Form;

fn main() -> ExitCode {
    let cli = cli::Cli::parse();

    let result = match cli.command {
        cli::Command::New(args) => commands::new::run(args),
        cli::Command::Forms => {
            list_forms();
            Ok(())
        }
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
    }
}
