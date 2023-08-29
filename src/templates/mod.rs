use crate::libs::types::{Content, User};
use clap::ValueEnum;
use json_value_merge::Merge;
use serde_json::Value;
use std::collections::HashMap;

pub mod common;
pub mod react;

#[derive(ValueEnum, Copy, Clone, Debug, PartialEq, Eq)]
pub enum Framework {
    None,
    React,
}

#[derive(Debug, Default, Clone)]
pub struct Template {
    pub scripts: Value,
    pub dependencies: Value,
    pub dev_dependencies: Value,
    pub tsconfig: Value,
    pub eslintrc: Value,
    pub readme: String,
    pub mit_license: String,
    pub gitignore: String,
    pub postcss_config: String,
    pub index_d: String,
    pub index: String,
    pub app: HashMap<String, Content>,
}

impl Template {
    fn merge(self, common: &Template) -> Template {
        let mut template: Template = common.clone();
        template.scripts.merge(self.scripts);
        template.dependencies.merge(self.dependencies);
        template.dev_dependencies.merge(self.dev_dependencies);
        template.tsconfig.merge(self.tsconfig);
        template.eslintrc.merge(self.eslintrc);
        template.app = self.app;
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
        }
        .get(&self.framework)
    }
}
