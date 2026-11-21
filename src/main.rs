mod cli;
mod commands;
mod generate;
mod github;
mod hash;
mod host;
mod mark;
mod model;
mod naming;
mod registries;
mod render;
mod repo;
mod rigger;
mod standard;
mod template;
mod templates;
mod turnout;
mod workflows;

use std::process::ExitCode;

use clap::Parser;

fn main() -> ExitCode {
    let cli = cli::Cli::parse();

    let result = match cli.command {
        cli::Command::New(args) => commands::new::run(*args),
        cli::Command::Init(args) => commands::init::run(*args),
        cli::Command::Adopt(args) => commands::adopt::run(*args),
        // A place left for a hand is not an error either: the files are
        // written, and the exit code says the upgrade is not finished.
        cli::Command::Upgrade(args) => match commands::upgrade::run(args) {
            Ok(true) => return ExitCode::SUCCESS,
            Ok(false) => return ExitCode::FAILURE,
            Err(e) => Err(e),
        },
        // A finding is an answer, not an error: the report has said what is
        // wrong, and the exit code is what a gate reads.
        cli::Command::Doctor(args) => match commands::doctor::run(args) {
            Ok(true) => return ExitCode::SUCCESS,
            Ok(false) => return ExitCode::FAILURE,
            Err(e) => Err(e),
        },
        cli::Command::Check(args) => match commands::check::run(args) {
            Ok(true) => return ExitCode::SUCCESS,
            Ok(false) => return ExitCode::FAILURE,
            Err(e) => Err(e),
        },
        // A setting GitHub refused is said in the report; the exit code says
        // the repository is not yet the way the project says.
        cli::Command::Repo(args) => match commands::repo::run(args) {
            Ok(true) => return ExitCode::SUCCESS,
            Ok(false) => return ExitCode::FAILURE,
            Err(e) => Err(e),
        },
        cli::Command::Open(args) => commands::open::run(args),
        cli::Command::Clean(args) => match commands::clean::run(args) {
            Ok(true) => return ExitCode::SUCCESS,
            Ok(false) => return ExitCode::FAILURE,
            Err(e) => Err(e),
        },
        cli::Command::Stand(command) => commands::stand::run(command),
        cli::Command::Ci(args) => match commands::ci::run(args) {
            Ok(true) => return ExitCode::SUCCESS,
            Ok(false) => return ExitCode::FAILURE,
            Err(e) => Err(e),
        },
        cli::Command::Assets(args) => commands::assets::run(args),
        // A taken code or colour is an answer too: the exit code says it.
        cli::Command::Brand(cli::BrandCommand::Pick(args)) => match commands::brand::pick(args) {
            Ok(true) => return ExitCode::SUCCESS,
            Ok(false) => return ExitCode::FAILURE,
            Err(e) => Err(e),
        },
        cli::Command::Stage(args) => commands::stage::run(args),
        cli::Command::Status(args) => commands::status::run(args),
        cli::Command::Versions(args) => commands::versions::run(args),
        cli::Command::Deps(args) => commands::deps::run(args),
        cli::Command::Msrv(args) => commands::msrv::run(args),
        cli::Command::Adr(command) => commands::adr::run(command),
        // A site that does not hold, or a transcript that no longer matches, is
        // an answer too: the report has said what, and the exit code says it.
        cli::Command::Docs(command) => match commands::docs::run(command) {
            Ok(true) => return ExitCode::SUCCESS,
            Ok(false) => return ExitCode::FAILURE,
            Err(e) => Err(e),
        },
        cli::Command::Readme(command) => match commands::readme::run(command) {
            Ok(true) => return ExitCode::SUCCESS,
            Ok(false) => return ExitCode::FAILURE,
            Err(e) => Err(e),
        },
        cli::Command::Changelog(args) => commands::changelog::run(args),
        cli::Command::Forms(args) => commands::forms::run(args),
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
