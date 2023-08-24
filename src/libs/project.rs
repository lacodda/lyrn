use crate::templates::{self, Framework, Template};
use clap::Args;
use std::error::Error;
use std::fs::{create_dir, File};
use std::io::Write;
use std::path::PathBuf;
use super::helpers::get_git_user;
use super::types::{Package, User};

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

pub fn create_project(args: CreateProjectArgs) -> Result<(), Box<dyn Error>> {
    let user: User = get_git_user()?;
    let name = args.name.to_string();
    let template = templates::get(&user, &args.framework);

    create_dir(&name)?;
    create_package(&name, &template)?;
    create_tsconfig(&name, &template)?;
    create_license(&name, &template)?;

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

fn create_tsconfig(name: &String, template: &Template) -> Result<(), Box<dyn Error>> {
    let file_path = PathBuf::from(name).join("tsconfig.json");
    let mut file = File::create(file_path)?;
    serde_json::to_writer_pretty(&mut file, &template.tsconfig)?;
    Ok(())
}

fn create_license(name: &String, template: &Template) -> Result<(), Box<dyn Error>> {
    let file_path = PathBuf::from(name).join("LICENSE");
    let mut file = File::create(file_path)?;
    file.write_all(template.mit_license.as_bytes())?;
    Ok(())
}