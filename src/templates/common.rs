use std::collections::HashMap;

use super::Template;
use crate::libs::{
    helpers::to_hash_map,
    types::{Tsconfig, TsconfigCompilerOptions},
};

pub fn get() -> Template {
    Template {
        scripts: to_hash_map(SCRIPTS),
        dependencies: to_hash_map(DEPENDENCIES),
        dev_dependencies: to_hash_map(DEV_DEPENDENCIES),
        tsconfig: tsconfig(),
        ..Template::default()
    }
}

const SCRIPTS: &[(&str, &str)] = &[("start", "lyrn start")];

const DEPENDENCIES: &[(&str, &str)] = &[("lyrn", "^1.0.0")];

const DEV_DEPENDENCIES: &[(&str, &str)] = &[
    ("@types/node", "^20.4.2"),
    ("@types/webpack-env", "^1.18.1"),
    ("@typescript-eslint/eslint-plugin", "^6.0.0"),
    ("@typescript-eslint/parser", "^6.0.0"),
    ("babel-loader", "^9.1.3"),
    ("copy-webpack-plugin", "^11.0.0"),
    ("css-loader", "^6.8.1"),
    ("dotenv", "^16.3.1"),
    ("eslint", "^8.45.0"),
    ("eslint-import-resolver-alias", "^1.1.2"),
    ("eslint-import-resolver-webpack", "^0.13.2"),
    ("eslint-plugin-import", "^2.27.5"),
    ("file-loader", "^6.2.0"),
    ("fork-ts-checker-webpack-plugin", "^8.0.0"),
    ("html-webpack-plugin", "^5.5.3"),
    ("mini-css-extract-plugin", "^2.7.6"),
    ("postcss-loader", "^7.3.3"),
    ("postcss-preset-env", "^9.0.0"),
    ("sass", "^1.63.6"),
    ("sass-loader", "^13.3.2"),
    ("serve", "^14.2.0"),
    ("style-loader", "^3.3.3"),
    ("ts-loader", "9.4.4"),
    ("typescript", "^5.1.6"),
    ("webpack", "^5.88.1"),
    ("webpack-cli", "^5.1.4"),
    ("webpack-dev-server", "^4.15.1"),
    ("webpack-shell-plugin-next", "^2.3.1"),
    ("zx", "^7.2.3"),
];

fn tsconfig() -> Tsconfig {
    Tsconfig {
        compile_on_save: Some(false),
        compiler_options: TsconfigCompilerOptions {
            root_dir: Some(".".into()),
            source_map: Some(true),
            declaration: Some(false),
            module_resolution: Some("node".into()),
            emit_decorator_metadata: Some(true),
            experimental_decorators: Some(true),
            import_helpers: Some(true),
            target: Some("ESNext".into()),
            module: Some("ESNext".into()),
            lib: vec!["ESNext".into(), "dom".into()],
            skip_lib_check: Some(true),
            skip_default_lib_check: Some(true),
            es_module_interop: Some(true),
            no_implicit_any: Some(true),
            resolve_json_module: Some(true),
            base_url: Some(".".into()),
            out_dir: Some("dist".into()),
            paths: HashMap::from([
                ("@/*".into(), vec!["src/*".into()]),
                ("@libs/ui".into(), vec!["../../libs/ui/src".into(), "libs/ui/src".into()]),
                ("@libs/utils".into(), vec!["../../libs/utils/src".into(), "libs/utils/src".into()]),
            ]),
            exclude: vec!["node_modules".into(), "dist".into()],
            ..TsconfigCompilerOptions::default()
        },
    }
}
