use super::helpers::is_default;
use crate::templates::{Framework, ProjectProps};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs::File;

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

pub fn project_config(project: Option<&ProjectProps>) -> ProjectConfig {
    let mut app = AppConfig { ..Default::default() };
    if project.is_some() {
        let project = project.unwrap();
        app.name = project.clone().name;
        app.framework = project.framework;
        app.title = project.name.to_uppercase();
    }
    ProjectConfig {
        app: app,
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
