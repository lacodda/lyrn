use crate::templates;
use clap::{Args, ValueEnum};
use serde::{Deserialize, Serialize, Serializer};
use std::collections::{BTreeMap, HashMap};
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

#[derive(ValueEnum, Copy, Clone, Debug, PartialEq, Eq)]
enum Framework {
    None,
    React,
}

#[derive(Debug, Default, Clone)]
struct Template {
    dependencies: HashMap<String, String>,
    dev_dependencies: HashMap<String, String>,
}

impl Template {
    fn merge(self, common: &Template) -> Template {
        let mut template: Template = common.clone();
        template.dependencies.extend(self.dependencies.into_iter());
        template.dev_dependencies.extend(self.dev_dependencies.into_iter());
        template
    }
}

#[derive(Debug, Default)]
struct Templates {
    common: Template,
    react: Template,
}

impl Templates {
    fn get(self, framework: &Framework) -> Template {
        match framework {
            Framework::None => self.common,
            Framework::React => self.react.merge(&self.common),
        }
    }
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

fn ordered_map<S, K: Ord + Serialize, V: Serialize>(value: &HashMap<K, V>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let ordered: BTreeMap<_, _> = value.iter().collect();
    ordered.serialize(serializer)
}

pub fn create_project(args: CreateProjectArgs) -> Result<(), Box<dyn Error>> {
    let name = args.name.to_string();
    let framework = args.framework;

    create_dir(&name)?;
    create_package(&name, &framework)?;

    Ok(())
}

fn create_package(_name: &String, _framework: &Framework) -> Result<(), Box<dyn Error>> {
    let common = Template {
        dependencies: to_hash_map(templates::common::DEPENDENCIES),
        dev_dependencies: to_hash_map(templates::common::DEV_DEPENDENCIES),
        ..Template::default()
    };

    let react = Template {
        dependencies: to_hash_map(templates::react::DEPENDENCIES),
        dev_dependencies: to_hash_map(templates::react::DEV_DEPENDENCIES),
        ..Template::default()
    };

    let templates = Templates { common: common, react: react }.get(_framework);

    let package = Package {
        name: _name.to_string(),
        version: "0.0.1".to_string(),
        description: "".to_string(),
        main: "src/main.ts".to_string(),
        scripts: to_hash_map(templates::common::SCRIPTS),
        keywords: vec!["app".to_string()],
        author: "author".to_string(),
        license: "MIT".to_string(),
        dependencies: templates.dependencies,
        dev_dependencies: templates.dev_dependencies,
    };

    let file_path = PathBuf::from(_name).join("package.json");
    let mut file = File::create(file_path)?;
    serde_json::to_writer_pretty(&mut file, &package)?;
    Ok(())
}

fn to_hash_map(value: &[(&str, &str)]) -> HashMap<String, String> {
    return value.into_iter().map(|(p, v)| (p.to_string(), v.to_string())).collect::<HashMap<_, _>>();
}
