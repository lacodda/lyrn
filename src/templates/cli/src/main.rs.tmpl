mod cli;
mod commands;
{{#self-update}}
mod update;
{{/self-update}}

use std::process::ExitCode;

use clap::Parser;

fn main() -> ExitCode {
    let cli = cli::Cli::parse();

    let result = match cli.command {
        cli::Command::Hello(args) => commands::hello::run(args),
{{#keyring}}
        cli::Command::Secret { command } => commands::secret::run(command),
{{/keyring}}
{{#self-update}}
        cli::Command::SelfUpdate => commands::self_update::run(),
{{/self-update}}
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            // The chain matters: the top line says what failed, the causes say
            // why. Printing only the outer error loses the actual reason.
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}
