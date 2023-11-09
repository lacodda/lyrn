use crate::libs::helpers::{clear_console, spinner_start};
use crate::libs::project_config::ProjectConfig;
use crate::tools::webpack;
use clap::Args;
use local_ip_address::local_ip;
use spinners::Spinner;
use std::error::Error;
use std::fs;
use std::io::{BufRead, Write};
use std::process::{Command, Stdio};
use std::thread;

#[derive(Debug, Args, Clone)]
#[command(args_conflicts_with_subcommands = true)]
pub struct StartArgs {
    script: Option<String>,
    #[arg(short, long)]
    pub port: Option<i32>,
}

pub fn cmd(start_args: StartArgs) -> Result<(), Box<dyn Error>> {
    let script = start_args.clone().script.or(Some("node_modules/lyrn/tools/webpack.js".into())).unwrap();
    if let Err(_) = fs::metadata(&script) {
        return Err(format!("File {} does not exist! Run the `start` command only in the project folder.", script).into());
    }
    let mut child = Command::new("node")
        .arg(&script)
        .arg("start")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn child process");

    let mut child_stdin: std::process::ChildStdin = child.stdin.take().expect("Failed to open stdin for child process");
    let mut spinner = spinner_start("Loading...").unwrap();
    let webpack_config = webpack::get_config_dev(&Some(start_args));
    let json_string = serde_json::to_string(&webpack_config).unwrap();
    child_stdin.write_all(&json_string.as_bytes()).expect("Failed to write to child process stdin");
    drop(child_stdin);
    let stdout = child.stdout.take().expect("Failed to open stdout for child process");

    thread::spawn(move || {
        let reader = std::io::BufReader::new(stdout);
        for line in reader.lines() {
            match line.unwrap().as_str() {
                "compile" => spinner = spinner_start("Loading...").unwrap(),
                "done" => done(&mut spinner, &webpack_config.project_config).unwrap(),
                _ => (),
            }
        }
    });
    let _ = child.wait();
    Ok(())
}

fn done(spinner: &mut Spinner, project_config: &ProjectConfig) -> Result<(), Box<dyn Error>> {
    spinner.stop();
    clear_console()?;
    let local_ip = local_ip().unwrap();
    println!("🚀 Your app running at:");
    println!("🔗 Local:    {}://{}:{}", &project_config.dev.protocol, &project_config.dev.host, &project_config.dev.port);
    println!("🔗 Network:  {}://{}:{}", &project_config.dev.protocol, &local_ip, &project_config.dev.port);
    Ok(())
}
