// fn main() {
//     if let Err(e) = lyrn::get_args().and_then(lyrn::run) {
//         eprintln!("{}", e);
//         std::process::exit(1);
//     }
// }

use ::clap::{Parser, Subcommand};

mod commands;

#[derive(Parser)]
#[command(author, version)]
#[command(
    about = "📦✨ Command line tool for rapid starting the development of your web application",
    long_about = "📦✨ Command line tool for rapid starting the development of your web application"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    CreateProject(commands::create::CreateProject),
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::CreateProject(project)) => commands::create::project(project),
        None => {}
    }
}
