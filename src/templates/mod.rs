use crate::libs::{
    project_config::ProjectConfig,
    types::{Content, User},
};
use crate::traits::value_ext::ValueExt;
use clap::ValueEnum;
use serde_json::Value;
use std::collections::HashMap;

pub mod common;
pub mod react;
pub mod vue;

#[derive(ValueEnum, Copy, Clone, Debug, PartialEq, Eq)]
pub enum Framework {
    None,
    React,
    Vue,
}

#[derive(Debug, Default, Clone)]
pub struct Template {
    pub scripts: Value,
    pub dependencies: Value,
    pub dev_dependencies: Value,
    pub project_config: ProjectConfig,
    pub tsconfig: Value,
    pub eslintrc: Value,
    pub readme: String,
    pub mit_license: String,
    pub gitignore: String,
    pub postcss_config: String,
    pub index_d: String,
    pub index: String,
    pub app: HashMap<&'static str, Content>,
}

impl Template {
    fn merge(self, common: &Template) -> Template {
        let mut template: Template = common.clone();
        template.scripts.merge_default(&self.scripts);
        template.dependencies.merge_default(&self.dependencies);
        template.dev_dependencies.merge_default(&self.dev_dependencies);
        template.tsconfig.merge_default(&self.tsconfig);
        template.eslintrc.merge_default(&self.eslintrc);
        template.app = self.app;
        template
    }
}

#[derive(Debug, Default)]
struct Templates {
    common: Template,
    react: Template,
    vue: Template,
}

impl Templates {
    fn get(self, framework: &Framework) -> Template {
        match framework {
            Framework::None => self.common,
            Framework::React => self.react.merge(&self.common),
            Framework::Vue => self.vue.merge(&self.common),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProjectProps {
    pub name: String,
    pub framework: Framework,
    pub user: User,
}

impl ProjectProps {
    pub fn get_template(self) -> Template {
        Templates {
            common: common::get(&self),
            react: react::get(&self),
            vue: vue::get(&self),
        }
        .get(&self.framework)
    }
}
