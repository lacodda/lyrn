use super::helpers::ordered_map;
use crate::templates::{self, Framework, Template};
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
    #[arg(
        long,
        short,
        default_value_t = Framework::None,
        value_enum
    )]
    framework: Framework,
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
    #[serde(serialize_with = "ordered_map")]
    dependencies: HashMap<String, String>,
    #[serde(serialize_with = "ordered_map")]
    dev_dependencies: HashMap<String, String>,
}

pub fn create_project(args: CreateProjectArgs) -> Result<(), Box<dyn Error>> {
    let name = args.name.to_string();
    let template = templates::get(&args.framework);

    create_dir(&name)?;
    create_package(&name, &template)?;

    Ok(())
}

fn create_package(name: &String, template: &Template) -> Result<(), Box<dyn Error>> {
    let package = Package {
        name: name.to_string(),
        version: "0.0.1".to_string(),
        description: "".to_string(),
        main: "src/main.ts".to_string(),
        scripts: template.scripts.to_owned(),
        keywords: vec!["app".to_string()],
        author: "author".to_string(),
        license: "MIT".to_string(),
        dependencies: template.dependencies.to_owned(),
        dev_dependencies: template.dev_dependencies.to_owned(),
    };

    let file_path = PathBuf::from(name).join("package.json");
    let mut file = File::create(file_path)?;
    serde_json::to_writer_pretty(&mut file, &package)?;
    Ok(())
}
