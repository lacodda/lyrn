use super::Template;
use crate::libs::{
    helpers::to_hash_map,
    types::{Tsconfig, TsconfigCompilerOptions, User},
};
use chrono::Datelike;
use std::collections::HashMap;

pub fn get(user: &User) -> Template {
    Template {
        scripts: to_hash_map(SCRIPTS),
        dependencies: to_hash_map(DEPENDENCIES),
        dev_dependencies: to_hash_map(DEV_DEPENDENCIES),
        tsconfig: tsconfig(),
        mit_license: mit_license(user),
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

fn mit_license(user: &User) -> String {
    let now = chrono::Utc::now();
    format!(
        r###"MIT License

Copyright (c) {} {} <{}>

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
"###,
        now.year(),
        user.name,
        user.email,
    )
}
