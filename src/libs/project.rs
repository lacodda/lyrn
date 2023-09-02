use super::helpers::get_git_user;
use super::types::{Content, Package};
use crate::libs::helpers::{clear_console, spinner_start};
use crate::templates::{Framework, ProjectProps, Template};
use clap::Args;
use std::collections::HashMap;
use std::env::set_current_dir;
use std::error::Error;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

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
    let project_props: ProjectProps = ProjectProps {
        name: args.name,
        framework: args.framework,
        user: get_git_user()?,
    };
    let template = project_props.clone().get_template();
    let mut project: HashMap<String, Content> = HashMap::new();
    project.insert("package.json".into(), Content::Pkg(package(&project_props, &template)));
    project.insert("tsconfig.json".into(), Content::Val(template.tsconfig));
    project.insert(".eslintrc.json".into(), Content::Val(template.eslintrc));
    project.insert("README.md".into(), Content::Str(template.readme));
    project.insert("LICENSE".into(), Content::Str(template.mit_license));
    project.insert(".gitignore".into(), Content::Str(template.gitignore));
    project.insert("postcss.config.js".into(), Content::Str(template.postcss_config));
    project.insert("src/index.d.ts".into(), Content::Str(template.index_d));
    project.insert("src/index.html".into(), Content::Str(template.index));
    project.extend(template.app);

    if args.show {
        return Ok(());
    }

    for (key, value) in project.clone().into_iter() {
        let mut path_vec: Vec<String> = key.split("/").map(|s| s.to_string()).collect();
        let file_name = path_vec.pop().unwrap();
        path_vec.insert(0, project_props.name.clone());
        let path = path_vec.join("/");
        let file_path = PathBuf::from(&path).join(&file_name);
        create_dir_all(&path)?;
        let mut file = File::create(file_path)?;
        match value {
            Content::Str(value) => file.write_all(value.as_bytes())?,
            Content::Val(value) => serde_json::to_writer_pretty(&mut file, &value)?,
            Content::Pkg(value) => serde_json::to_writer_pretty(&mut file, &value)?,
        }
    }

    set_current_dir(&project_props.name).expect("Failed to change working directory");
    run_npm_install(&project_props.name);

    Ok(())
}

fn package(project_props: &ProjectProps, template: &Template) -> Package {
    Package {
        name: project_props.name.to_owned(),
        version: "0.0.1".to_string(),
        description: "".to_string(),
        main: "src/main.ts".to_string(),
        scripts: template.scripts.to_owned(),
        keywords: vec!["app".to_string()],
        author: project_props.user.name.to_owned(),
        license: "MIT".to_string(),
        dependencies: template.dependencies.to_owned(),
        dev_dependencies: template.dev_dependencies.to_owned(),
    }
}

fn run_npm_install(name: &String) {
    #[cfg(windows)]
    pub const NPM: &'static str = "npm.cmd";

    #[cfg(not(windows))]
    pub const NPM: &'static str = "npm";

    let mut spinner = spinner_start("📦 Installing npm packages...").unwrap();
    let output = Command::new(NPM).arg("install").output().expect("Failed to execute 'npm install'");
    spinner.stop();
    clear_console().unwrap();

    if output.status.success() {
        println!("npm packages installed successfully 👍");
        println!("you can go to the project directory \"cd {}\"", name);
        println!("and run it 🚀 with the \"npm start\" command");
    } else {
        println!("npm install failed with error:");
        println!("{}", String::from_utf8_lossy(&output.stderr));
    }
}
