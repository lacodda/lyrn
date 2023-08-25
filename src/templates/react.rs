use super::Template;
use crate::libs::types::{Eslintrc, Tsconfig, TsconfigCompilerOptions};
use serde_json::{json, Value};
use std::collections::HashMap;

pub fn get() -> Template {
    Template {
        dependencies: dependencies(),
        dev_dependencies: dev_dependencies(),
        tsconfig: tsconfig(),
        eslintrc: eslintrc(),
        ..Template::default()
    }
}

fn dependencies() -> Value {
    json!({
        "react": "^18.2.0",
        "react-dom": "^18.2.0",
        "react-router-dom": "^6.13.0",
        "styled-components": "^6.0.7"
    })
}

fn dev_dependencies() -> Value {
    json!({
        "@pmmmwh/react-refresh-webpack-plugin": "^0.5.10",
        "@types/react": "^18.2.7",
        "@types/react-dom": "^18.2.4",
        "eslint-plugin-react": "^7.32.2",
        "react-refresh": "^0.14.0",
    })
}

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
