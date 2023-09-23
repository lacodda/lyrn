use crate::libs::helpers::{clear_console, spinner_start, convert_bytes};
use crate::tools::webpack;
use clap::Args;
use serde_json::{from_str, Value};
use spinners::Spinner;
use std::error::Error;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::thread;

#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct BuildArgs {
    script: Option<String>,
}

pub fn cmd(build_args: BuildArgs) -> Result<(), Box<dyn Error>> {
    let script = build_args.script.or(Some("node_modules/lyrn/tools/webpack.js".into())).unwrap();
    let dist_dir = "dist";

    if let Err(_) = fs::metadata(&script) {
        return Err(format!("File {} does not exist! Run the `build` command only in the project folder.", script).into());
    }

    if fs::metadata(&dist_dir).is_ok() {
        fs::remove_dir_all(&dist_dir)?;
    }

    let mut child = Command::new("node")
        .arg(&script)
        .arg("build")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn child process");

    let mut child_stdin: std::process::ChildStdin = child.stdin.take().expect("Failed to open stdin for child process");
    let mut spinner = spinner_start("Loading...").unwrap();
    let webpack_config = webpack::get_config_prod();
    let json_string = serde_json::to_string(&webpack_config).unwrap();
    child_stdin.write_all(&json_string.as_bytes()).expect("Failed to write to child process stdin");
    drop(child_stdin);
    let stdout = child.stdout.expect("Failed to open stdout for child process");

    let handle = thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            match line.unwrap().as_str() {
                done_str if done_str.starts_with("done") => done(&mut spinner, &done_str).unwrap(),
                _ => (),
            }
        }
    });
    handle.join().unwrap();

    Ok(())
}

fn done(spinner: &mut Spinner, done_str: &str) -> Result<(), Box<dyn Error>> {
    let json_str = done_str.replace("done ", "");
    let json_value: Value = from_str(json_str.as_str())?;
    let assets = json_value["assets"].as_array().unwrap();
    
    spinner.stop();
    clear_console()?;

    println!("✔️ Application build completed!");
    println!("");
    println!("{:60} {:10}", "File", "Size");
    println!("");

    for (_index, item) in assets.iter().enumerate() {
        println!("{:60} {:10}", item["name"].as_str().unwrap(), convert_bytes(item["size"].as_u64().unwrap()));
    }

    Ok(())
}
