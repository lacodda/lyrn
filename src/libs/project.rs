use crate::templates::common;
use clap::Args;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fs::{create_dir, File};
use std::path::PathBuf;

#[derive(Debug, Args)]
pub struct CreateProjectArgs {
    #[arg(required = true)]
    name: String,
    #[arg(short, long)]
    framework: Option<String>,
    #[arg(short, long)]
    tool: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
    dev_dependencies: HashMap<String, String>,
}

pub fn create_project(args: CreateProjectArgs) -> Result<(), Box<dyn Error>> {
    let name = args.name.to_string();
    create_dir(&name)?;
    create_package(&name)?;
    Ok(())
}

fn create_package(_name: &String) -> Result<(), Box<dyn Error>> {
    let package = Package {
        name: _name.to_string(),
        version: "0.0.1".to_string(),
        description: "".to_string(),
        main: "src/main.ts".to_string(),
        scripts: to_hash_map(common::SCRIPTS),
        keywords: vec!["app".to_string()],
        author: "author".to_string(),
        license: "MIT".to_string(),
        dependencies: to_hash_map(common::DEPENDENCIES),
        dev_dependencies: to_hash_map(common::DEV_DEPENDENCIES),
    };

    let file_path = PathBuf::from(_name).join("package.json");
    let mut file = File::create(file_path)?;
    serde_json::to_writer_pretty(&mut file, &package)?;
    Ok(())
}

fn to_hash_map(value: &[(&str, &str)]) -> HashMap<String, String> {
    return value
        .into_iter()
        .map(|(p, v)| (p.to_string(), v.to_string()))
        .collect::<HashMap<_, _>>();
}
