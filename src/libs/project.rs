use super::helpers::get_git_user;
use super::project_config::PROJECT_CONFIG;
use super::types::{Content, Package};
use crate::libs::helpers::{clear_console, spinner_start};
use crate::templates::{Framework, ProjectProps, Template};
use clap::Args;
use serde_json::json;
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
    let mut project: HashMap<&str, Content> = HashMap::new();
    project.insert(PROJECT_CONFIG, Content::Val(json!(template.project_config)));
    project.insert("package.json", Content::Pkg(package(&project_props, &template)));
    project.insert("tsconfig.json", Content::Val(template.tsconfig));
    project.insert(".eslintrc.json", Content::Val(template.eslintrc));
    project.insert("README.md", Content::Str(template.readme));
    project.insert("LICENSE", Content::Str(template.mit_license));
    project.insert(".gitignore", Content::Str(template.gitignore));
    project.insert("postcss.config.js", Content::Str(template.postcss_config));
    project.insert("src/index.d.ts", Content::Str(template.index_d));
    project.insert("src/index.html", Content::Str(template.index));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::libs::types::User;
    use crate::tools::webpack::config_file;
    use std::env::current_dir;
    use std::fs;
    use std::path::Path;
    use tempfile::tempdir;

    #[test]
    fn test_package() {
        // Create a sample ProjectProps
        let project_props = ProjectProps {
            name: "my_project".to_string(),
            user: User {
                name: "John Doe".to_string(),
                email: "john@example.com".to_string(),
            },
            framework: Framework::None,
        };

        // Create a sample Template
        let template = Template {
            scripts: json!({
                "start": "start",
                "build": "build",
            }),
            dependencies: json!({
                "dep1": "1.0.0",
                "dep2": "1.0.0",
            }),
            dev_dependencies: json!({
                "dev_dep1": "1.0.0",
                "dev_dep2": "1.0.0",
            }),
            project_config: config_file("").unwrap(),
            tsconfig: json!({}),
            eslintrc: json!({}),
            readme: "readme".to_string(),
            mit_license: "mit_license".to_string(),
            gitignore: "gitignore".to_string(),
            postcss_config: "postcss_config".to_string(),
            index_d: "index_d".to_string(),
            index: "index".to_string(),
            app: HashMap::new(),
        };

        // Call the package function
        let result = package(&project_props, &template);

        // Check that the Package struct fields match the expected values
        assert_eq!(result.name, "my_project");
        assert_eq!(result.version, "0.0.1");
        assert_eq!(result.description, "");
        assert_eq!(result.main, "src/main.ts");
        assert_eq!(result.scripts, template.scripts);
        assert_eq!(result.keywords, vec!["app".to_string()]);
        assert_eq!(result.author, "John Doe");
        assert_eq!(result.license, "MIT");
        assert_eq!(result.dependencies, template.dependencies);
        assert_eq!(result.dev_dependencies, template.dev_dependencies);
    }

    #[test]
    fn test_run_npm_install() {
        // Create a temporary directory for the test project
        let temp_dir = tempdir().unwrap();
        let project_dir = temp_dir.path().join("test_project");
        fs::create_dir(&project_dir).unwrap();

        // Copy the package.json file from the example to the test directory
        let package_json_path = Path::new("examples/package.json");
        fs::copy(package_json_path, project_dir.join("package.json")).unwrap();

        // Change to the test directory and call the run_npm_install function
        let current_dir = current_dir().unwrap();
        set_current_dir(&project_dir).unwrap();
        let project_name = project_dir.file_name().unwrap().to_str().unwrap();
        run_npm_install(&project_name.to_string());

        // Check that the test directory now contains a node_modules folder with installed packages
        assert!(project_dir.join("node_modules").exists());

        // Return to the original directory and delete the temporary directory
        set_current_dir(current_dir).unwrap();
        temp_dir.close().unwrap();
    }
}
