use clap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

#[derive(clap::Args)]
pub struct CreateProject {
    pub name: Option<String>,
    pub framework: Option<String>,
    pub tool: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct Package {
    name: String,
    version: String,
    description: String,
    main: String,
    scripts: HashMap<String, String>,
    keywords: Vec<String>,
    author: String,
    license: String,
    dependencies: HashMap<String, String>,
}

pub fn add(input: &String) -> std::io::Result<()> {
    fs::create_dir(input)?;
    Ok(())
}

pub fn project(project: &CreateProject) {
    match project.name {
        Some(ref _name) => {
            let _res = self::add(_name);
        }
        None => {
            println!("Please provide a project name");
        }
    }
}
