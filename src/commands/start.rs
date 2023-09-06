use crate::libs::helpers::{clear_console, spinner_start};
use crate::tools::webpack::{self, AppConfig};
use clap::Args;
use local_ip_address::local_ip;
use spinners::Spinner;
use std::error::Error;
use std::fs;
use std::io::{BufRead, Write};
use std::process::{Command, Stdio};
use std::thread;

#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct StartArgs {
    script: Option<String>,
}

pub fn cmd(start_args: StartArgs) -> Result<(), Box<dyn Error>> {
    let script = start_args.script.or(Some("node_modules/lyrn/tools/webpack.js".into())).unwrap();
    if let Err(_) = fs::metadata(&script) {
        return Err(format!("File {} does not exist! Run the `start` command only in the project folder.", script).into());
    }
    let mut child = Command::new("node")
        .arg(&script)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn child process");

    let mut child_stdin: std::process::ChildStdin = child.stdin.take().expect("Failed to open stdin for child process");
    let mut spinner = spinner_start("Loading...").unwrap();
    let webpack = webpack::get();
    let json_string = serde_json::to_string(&webpack).unwrap();
    child_stdin.write_all(&json_string.as_bytes()).expect("Failed to write to child process stdin");
    drop(child_stdin);
    let stdout = child.stdout.take().expect("Failed to open stdout for child process");

    thread::spawn(move || {
        let reader = std::io::BufReader::new(stdout);
        for line in reader.lines() {
            match line.unwrap().as_str() {
                "compile" => spinner = spinner_start("Loading...").unwrap(),
                "done" => done(&mut spinner, &webpack.app_config).unwrap(),
                _ => (),
            }
        }
    });
    let _ = child.wait();
    Ok(())
}

fn done(spinner: &mut Spinner, app_config: &AppConfig) -> Result<(), Box<dyn Error>> {
    spinner.stop();
    clear_console()?;
    let local_ip = local_ip().unwrap();
    println!("🚀 Your app running at:");
    println!("🔗 Local:    {}://{}:{}", &app_config.protocol, &app_config.host, &app_config.port);
    println!("🔗 Network:  {}://{}:{}", &app_config.protocol, &local_ip, &app_config.port);
    Ok(())
}
