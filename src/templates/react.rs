use super::Template;
use crate::libs::{
    helpers::to_hash_map,
    types::{Eslintrc, Tsconfig, TsconfigCompilerOptions},
};
use serde_json::json;
use std::collections::HashMap;

pub fn get() -> Template {
    Template {
        dependencies: to_hash_map(DEPENDENCIES),
        dev_dependencies: to_hash_map(DEV_DEPENDENCIES),
        tsconfig: tsconfig(),
        eslintrc: eslintrc(),
        ..Template::default()
    }
}

const DEPENDENCIES: &[(&str, &str)] = &[
    ("react", "^18.2.0"),
    ("react-dom", "^18.2.0"),
    ("react-router-dom", "^6.13.0"),
    ("styled-components", "^6.0.7"),
];

const DEV_DEPENDENCIES: &[(&str, &str)] = &[
    ("@pmmmwh/react-refresh-webpack-plugin", "^0.5.10"),
    ("@types/react", "^18.2.7"),
    ("@types/react-dom", "^18.2.4"),
    ("eslint-plugin-react", "^7.32.2"),
    ("react-refresh", "^0.14.0"),
];

fn tsconfig() -> Tsconfig {
    Tsconfig {
        compiler_options: TsconfigCompilerOptions {
            jsx: Some("react".into()),
            ..TsconfigCompilerOptions::default()
        },
        ..Tsconfig::default()
    }
}

fn eslintrc() -> Eslintrc {
    Eslintrc {
        extends: Some(vec!["plugin:react/recommended".into()]),
        rules: Some(HashMap::from([("react/prop-types".into(), json!("off"))])),
        ..Eslintrc::default()
    }
}
