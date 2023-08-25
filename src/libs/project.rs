use super::helpers::get_git_user;
use super::types::{Package, User};
use crate::templates::{self, Framework, Template};
use clap::Args;
use std::error::Error;
use std::fs::{create_dir, create_dir_all, File};
use std::io::Write;
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
    #[arg(short, long)]
    show: bool,
}

pub fn create_project(args: CreateProjectArgs) -> Result<(), Box<dyn Error>> {
    let user: User = get_git_user()?;
    let name = args.name.to_string();
    let template = templates::get(&user, &args.framework);
    if args.show {
        return Ok(());
    }
    
    create_dir(&name)?;
    create_package(&name, &template)?;
    create_tsconfig(&name, &template)?;
    create_eslintrc(&name, &template)?;
    create_license(&name, &template)?;
    create_app(&name, &template)?;

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

fn create_eslintrc(name: &String, template: &Template) -> Result<(), Box<dyn Error>> {
    let file_path = PathBuf::from(name).join(".eslintrc.json");
    let mut file = File::create(file_path)?;
    serde_json::to_writer_pretty(&mut file, &template.eslintrc)?;
    Ok(())
}

fn create_license(name: &String, template: &Template) -> Result<(), Box<dyn Error>> {
    let file_path = PathBuf::from(name).join("LICENSE");
    let mut file = File::create(file_path)?;
    file.write_all(template.mit_license.as_bytes())?;
    Ok(())
}

fn create_app(name: &String, template: &Template) -> Result<(), Box<dyn Error>> {
    for (key, value) in template.app.clone().into_iter() {
        let mut path_vec: Vec<String> =  key.split("/").map(|s| s.to_string()).collect();
        let file_name = path_vec.pop().unwrap();
        path_vec.insert(0, name.to_string());
        let path = path_vec.join("/");
        create_dir_all(&path)?;
        let file_path = PathBuf::from(&path).join(&file_name);
        let mut file = File::create(file_path)?;
        file.write_all(value.as_bytes())?;
    }

    Ok(())
}
