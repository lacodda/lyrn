use super::Template;
use crate::libs::helpers::to_hash_map;

pub fn get() -> Template {
    Template {
        scripts: to_hash_map(SCRIPTS),
        dependencies: to_hash_map(DEPENDENCIES),
        dev_dependencies: to_hash_map(DEV_DEPENDENCIES),
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
