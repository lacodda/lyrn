use crate::libs::types::{Tsconfig, User};
use clap::ValueEnum;
use json_value_merge::Merge;
use serde_json::Value;

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
    pub tsconfig: Tsconfig,
    pub eslintrc: Value,
    pub mit_license: String,
}

impl Template {
    fn merge(self, common: &Template) -> Template {
        let mut template: Template = common.clone();
        template.scripts.merge(self.scripts);
        template.dependencies.merge(self.dependencies);
        template.dev_dependencies.merge(self.dev_dependencies);
        template.eslintrc.merge(self.eslintrc);
        template.tsconfig = self.tsconfig.merge(&common.tsconfig);
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

pub fn get(user: &User, framework: &Framework) -> Template {
    Templates {
        common: common::get(user),
        react: react::get(),
    }
    .get(framework)
}
