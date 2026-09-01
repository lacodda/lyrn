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
    for form in Form::ALL {
        println!("{:<8} {}", form.as_str(), form.summary());
    }
}
