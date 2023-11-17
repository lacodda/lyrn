use json_value_merge::Merge;
use path_absolutize::*;
use serde::{Deserialize, Serialize};
use serde_json::{from_str, json, Value};
use std::{
    env,
    error::Error,
    fs,
    path::{Path, PathBuf},
    string::String,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Aliases {
    pub src: String,
    pub build: String,
    pub public: String,
    pub images: String,
    pub main: String,
}

impl Aliases {
    fn iter_mut(&mut self) -> impl Iterator<Item = &mut String> {
        IntoIterator::into_iter([&mut self.src, &mut self.build, &mut self.public, &mut self.images, &mut self.main])
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectAliases {
    pub aliases: Aliases,
    pub is_abs_path: bool,
}

impl ProjectAliases {
    pub fn get(&mut self) -> Aliases {
        self.aliases.iter_mut().for_each(|field| {
            match self.is_abs_path {
                true => *field = Self::get_abs_path(&*field),
                false => *field = format!(">>>path.resolve(cwd, '{}')", field)
            }
        });
        self.to_owned().aliases
    }

    pub fn get_json(&mut self) -> Value {
        let mut aliases_json = json!(self.aliases);
        if let Ok(tsconfig_paths) = Self::ts_config_paths("tsconfig.json") {
            aliases_json.merge(tsconfig_paths);
        }
        aliases_json
    }

    fn get_abs_path(alias: &String) -> String {
        let cwd = env::current_dir().unwrap();
        let mut path = PathBuf::from(&cwd);
        let path_vec: Vec<String> = alias.split("/").map(|s| s.to_string()).collect();
        path.extend(&path_vec);
        path.to_string_lossy().into_owned()
    }

    fn ts_config_paths(filename: &str) -> Result<Value, Box<dyn Error>> {
        let mut config_paths = json!({});
        let data = fs::read_to_string(PathBuf::from(&filename))?;
        let json: Value = from_str(&data)?;
        let mut paths: Value = json["compilerOptions"]["paths"].to_owned();
        if paths.is_null() {
            paths = json!({});
        }
        paths.as_object().iter().flat_map(|s| s.iter()).for_each(|(key, value)| {
            let path_str = value[0].as_str().unwrap().to_string().replace("/*", "");
            let path = Path::new(path_str.as_str());
            config_paths.merge(json!({
              key.replace("/*", ""):
              path.absolutize().unwrap().to_str().unwrap()
            }))
        });
        let extends = &json["extends"];
        if extends.is_string() {
            let extends = json["extends"].as_str().unwrap();
            let extended_paths = Self::ts_config_paths(extends)?;
            config_paths.merge(extended_paths);
        }

        Ok(config_paths)
    }
}
