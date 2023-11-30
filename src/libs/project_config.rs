use super::helpers::is_default;
use crate::commands::start::StartArgs;
use crate::templates::{Framework, ProjectProps};
use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string};
use std::error::Error;
use std::fs::{read_to_string, File};

pub const PROJECT_CONFIG: &str = "lyrn.json";

pub enum EnvType {
    Dev,
    Prod,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    #[serde(default)]
    pub app: AppConfig,
    #[serde(default)]
    pub dev: DevConfig,
    #[serde(default)]
    pub prod: ProdConfig,
}

impl ProjectConfig {
    pub fn set_config(&mut self, env_type: &EnvType, config: &str) -> &mut ProjectConfig {
        match env_type {
            EnvType::Dev => self.dev.config = config.into(),
            EnvType::Prod => self.prod.config = config.into(),
        }
        self
    }

    pub fn save(&mut self) -> Result<(), Box<dyn Error>> {
        let file = File::create(PROJECT_CONFIG)?;
        serde_json::to_writer_pretty(&file, &self)?;
        Ok(())
    }

    pub fn get(start_args: &Option<StartArgs>) -> Self {
        let data = read_to_string(PROJECT_CONFIG).unwrap_or(to_string(&Self::default()).unwrap());
        let mut project_config: Self = from_str(&data).unwrap_or_default();
        if start_args.is_some() {
            let start_args = start_args.clone().unwrap();
            if start_args.port.is_some() {
                project_config.dev.port = start_args.port.unwrap()
            }
        }
        project_config
    }

    pub fn create(project_props: &ProjectProps) -> Self {
        let mut project_config = Self::default();
        project_config.app = AppConfig {
            name: project_props.clone().name,
            title: project_props.name.to_uppercase(),
            framework: project_props.framework,
        };
        project_config
    }

    pub fn default() -> Self {
        Self {
            app: AppConfig { ..Default::default() },
            dev: DevConfig {
                public_path: "/".into(),
                protocol: "http".into(),
                host: "localhost".into(),
                port: 8080,
                ..Default::default()
            },
            prod: ProdConfig {
                public_path: "/".into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default, skip_serializing_if = "is_default")]
    pub name: String,
    #[serde(default, skip_serializing_if = "is_default")]
    pub title: String,
    #[serde(default, skip_serializing_if = "is_default")]
    pub framework: Framework,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DevConfig {
    #[serde(default, skip_serializing_if = "is_default")]
    pub public_path: String,
    #[serde(default, skip_serializing_if = "is_default")]
    pub protocol: String,
    #[serde(default, skip_serializing_if = "is_default")]
    pub host: String,
    #[serde(default, skip_serializing_if = "is_default")]
    pub port: i32,
    #[serde(default, skip_serializing_if = "is_default")]
    pub config: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ProdConfig {
    #[serde(default, skip_serializing_if = "is_default")]
    pub public_path: String,
    #[serde(default, skip_serializing_if = "is_default")]
    pub config: String,
}
