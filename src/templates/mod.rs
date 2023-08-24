use crate::libs::types::{Tsconfig, User};
use clap::ValueEnum;
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
    pub scripts: HashMap<String, String>,
    pub dependencies: HashMap<String, String>,
    pub dev_dependencies: HashMap<String, String>,
    pub tsconfig: Tsconfig,
    pub mit_license: String,
}

impl Template {
    fn merge(self, common: &Template) -> Template {
        let mut template: Template = common.clone();
        template.scripts.extend(self.scripts.into_iter());
        template.dependencies.extend(self.dependencies.into_iter());
        template.dev_dependencies.extend(self.dev_dependencies.into_iter());
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
