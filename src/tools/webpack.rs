use crate::{
    commands::start::StartArgs,
    templates::{common::project_config as default_project_config, ProjectConfig},
};
use json_value_merge::Merge;
use path_absolutize::*;
use serde::{Deserialize, Serialize};
use serde_json::{from_str, from_value, json, to_string, Value};
use std::{
    env,
    error::Error,
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebpackConfig {
    pub project_config: ProjectConfig,
    pub config: Value,
    pub constants: Vec<String>,
    pub plugins: Vec<String>,
    pub rules: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Aliases {
    src: String,
    build: String,
    public: String,
    images: String,
    main: String,
}

impl Aliases {
    fn iter_mut(&mut self) -> impl Iterator<Item = &mut String> {
        IntoIterator::into_iter([&mut self.src, &mut self.build, &mut self.public, &mut self.images, &mut self.main])
    }
}

pub fn get_config_dev(start_args: &Option<StartArgs>) -> WebpackConfig {
    WebpackConfig {
        project_config: project_config(start_args),
        config: config_dev(start_args),
        constants: vec![
            PATH_CONST.into(),
            WEBPACK_CONST.into(),
            FORK_TS_CHECKER_WEBPACK_PLUGIN_CONST.into(),
            COPY_WEBPACK_PLUGIN_CONST.into(),
            HTML_WEBPACK_PLUGIN_CONST.into(),
            REACT_REFRESH_WEBPACK_PLUGIN_CONST.into(),
            PROCESS_CWD_CONST.into(),
        ],
        plugins: vec![
            fork_ts_checker_webpack_plugin(),
            copy_webpack_plugin(aliases()),
            html_webpack_plugin(),
            hot_module_replacement_plugin(),
            react_refresh_webpack_plugin(),
        ],
        rules: vec![tsx_rule(), style_rule(true), images_rule(), inline_rule()],
    }
}

pub fn get_config_prod() -> WebpackConfig {
    WebpackConfig {
        project_config: project_config(&None),
        config: config_prod(),
        constants: vec![
            PATH_CONST.into(),
            WEBPACK_CONST.into(),
            FORK_TS_CHECKER_WEBPACK_PLUGIN_CONST.into(),
            COPY_WEBPACK_PLUGIN_CONST.into(),
            HTML_WEBPACK_PLUGIN_CONST.into(),
            MINI_CSS_EXTRACT_PLUGIN_CONST.into(),
            REACT_REFRESH_WEBPACK_PLUGIN_CONST.into(),
            PROCESS_CWD_CONST.into(),
        ],
        plugins: vec![
            fork_ts_checker_webpack_plugin(),
            copy_webpack_plugin(aliases()),
            html_webpack_plugin(),
            mini_css_extract_plugin(),
            hot_module_replacement_plugin(),
            react_refresh_webpack_plugin(),
        ],
        rules: vec![tsx_rule(), style_rule(false), images_rule(), inline_rule()],
    }
}

const PROCESS_CWD_CONST: &str = "cwd = process.cwd();";
const PATH_CONST: &str = "path = require('path');";
const WEBPACK_CONST: &str = "webpack = require('webpack');";
const FORK_TS_CHECKER_WEBPACK_PLUGIN_CONST: &str = "ForkTsCheckerWebpackPlugin = require('fork-ts-checker-webpack-plugin');";
const HTML_WEBPACK_PLUGIN_CONST: &str = "HtmlWebpackPlugin = require('html-webpack-plugin');";
const MINI_CSS_EXTRACT_PLUGIN_CONST: &str = "MiniCssExtractPlugin = require('mini-css-extract-plugin');";
const COPY_WEBPACK_PLUGIN_CONST: &str = "CopyWebpackPlugin = require('copy-webpack-plugin');";
const REACT_REFRESH_WEBPACK_PLUGIN_CONST: &str = "ReactRefreshWebpackPlugin = require('@pmmmwh/react-refresh-webpack-plugin');";

fn get_abs_path(alias: &String) -> String {
    let cwd = env::current_dir().unwrap();
    let mut path = PathBuf::from(&cwd);
    let path_vec: Vec<String> = alias.split("/").map(|s| s.to_string()).collect();
    path.extend(&path_vec);
    path.to_string_lossy().into_owned()
}

fn aliases() -> Aliases {
    let mut aliases = Aliases {
        src: "src".into(),
        build: "dist".into(),
        public: "public".into(),
        images: "src/images".into(),
        main: "src/main.ts".into(),
    };
    aliases.iter_mut().for_each(|field| {
        *field = get_abs_path(&*field);
    });
    aliases
}

fn aliases_json() -> Value {
    let mut aliases_json = json!(aliases());
    aliases_json.merge(ts_config_paths("tsconfig.json").unwrap());
    aliases_json
}

fn ts_config_paths(filename: &str) -> Result<Value, Box<dyn Error>> {
    let mut config_paths = json!({});
    let data = fs::read_to_string(PathBuf::from(&filename))?;
    let json: Value = from_str(&data)?;
    let mut paths: Value = json["compilerOptions"]["paths"].to_owned();
    if paths.is_null() {
        paths = json!({});
    }
    paths.as_object().iter().flat_map(|s| s.iter()).for_each(|(key, value)| {
        let path_str = value[0].as_str().unwrap().to_string().replace("/*", "");
        let path = Path::new(path_str.as_str());
        config_paths.merge(json!({
          key.replace("/*", ""):
          path.absolutize().unwrap().to_str().unwrap()
        }))
    });
    let extends = &json["extends"];
    if extends.is_string() {
        let extends = json["extends"].as_str().unwrap();
        let extended_paths = ts_config_paths(extends)?;
        config_paths.merge(extended_paths);
    }

    Ok(config_paths)
}

pub fn config_file(file_name: &str) -> Result<ProjectConfig, Box<dyn Error>> {
    let data = fs::read_to_string(PathBuf::from(file_name)).unwrap_or(to_string(&default_project_config()).unwrap());
    let config: ProjectConfig = from_str(&data).unwrap_or_default();

    Ok(config)
}

fn project_config(start_args: &Option<StartArgs>) -> ProjectConfig {
    let config_file = json!(config_file("lyrn.json").unwrap());
    let mut project_config = json!(default_project_config());
    project_config.merge(config_file);
    let mut project_config: ProjectConfig = from_value(project_config).unwrap();
    match start_args {
        Some(start_args) => {
            if start_args.port.is_some() {
                project_config.dev.port = start_args.port.unwrap();
            }
        }
        None => (),
    }
    project_config
}

pub fn config_dev(start_args: &Option<StartArgs>) -> Value {
    let config = project_config(start_args);
    json!({
        "mode": "development",
        "entry": [aliases().main],
        "output": {
          "path": aliases().build,
          "publicPath": format!("{}://{}:{}/", config.dev.protocol, config.dev.host, config.dev.port),
          "filename": "js/[name].[contenthash].bundle.js",
          "assetModuleFilename": "assets/[hash][ext][query]",
        },
        "resolve": {
          "modules": [aliases().src, "node_modules"],
          "extensions": [".tsx", ".ts", ".mjs", ".js", ".jsx", ".json", ".wasm", ".css"],
          "alias": aliases_json(),
        },
        "module": {
          "rules": [],
        },
        "plugins": [],
        "stats": "errors-warnings",
        "devtool": "inline-source-map",
        "optimization": {
          "minimize": false,
        },
        "performance": {
          "hints": false,
        },
        "cache": false,
        "target": "web",
        "devServer": {
          "historyApiFallback": true,
          "compress": true,
          "hot": true,
          "port": config.dev.port,
          "static": "./",
          "headers": {
            "Access-Control-Allow-Origin": "*",
            "Access-Control-Allow-Methods": "GET, POST, PUT, DELETE, PATCH, OPTIONS",
            "Access-Control-Allow-Headers": "X-Requested-With, content-type, Authorization",
          },
        },
        "infrastructureLogging": {
          "level": "warn",
        },
        "stats": {
          "assets": false,
          "modules": false,
        },
      }
    )
}

pub fn config_prod() -> Value {
    let config = project_config(&None);
    json!({
        "mode": "production",
        "entry": [aliases().main],
        "output": {
          "path": aliases().build,
          "publicPath": config.prod.public_path,
          "filename": "js/[name].[contenthash].bundle.js",
          "assetModuleFilename": "assets/[hash][ext][query]",
          "chunkFilename": "js/[name].[chunkhash].chunk.js",
          "clean": true,
        },
        "resolve": {
          "modules": [aliases().src, "node_modules"],
          "extensions": [".tsx", ".ts", ".mjs", ".js", ".jsx", ".json", ".wasm", ".css"],
          "alias": aliases_json(),
        },
        "module": {
          "rules": [],
        },
        "plugins": [],
        "stats": "errors-warnings",
        "devtool": false,
        "optimization": {
          "minimize": false,
          "sideEffects": true,
          "concatenateModules": true,
        },
        "performance": {
          "hints": false,
          "maxEntrypointSize": 512000,
          "maxAssetSize": 512000,
        },
      }
    )
}

fn tsx_rule() -> String {
    r###"new Object({
  test: /\.tsx?$/,
  exclude: /(node_modules|\.webpack)/,
  use: {
    loader: 'ts-loader',
    options: {
      transpileOnly: true,
    },
  },
});
"###
    .into()
}

fn style_rule(is_dev: bool) -> String {
    format!(
        r###"let isDev = {};
new Object({{
  test: /\.(sass|scss|css)$/,
  use: [
    {{ loader: isDev ? 'style-loader' : MiniCssExtractPlugin.loader }},
    {{
      loader: 'css-loader',
      options: {{
        importLoaders: isDev ? 1 : 2,
        sourceMap: isDev,
        modules: isDev,
      }},
    }},
    {{ loader: 'postcss-loader', options: {{ sourceMap: isDev }} }},
    {{ loader: 'sass-loader', options: {{ sourceMap: isDev }} }},
  ],
}});
"###,
        { is_dev }
    )
}

fn images_rule() -> String {
    r###"new Object({
  test: /\.(?:ico|gif|png|jpe?g)$/i,
  type: 'asset/resource',
  generator: {
    filename: 'assets/[hash][ext][query]',
  },
});
"###
    .into()
}

fn inline_rule() -> String {
    r###"new Object({
  test: /\.(woff(2)?|eot|ttf|otf|svg|)$/i,
  type: 'asset/inline',
});
"###
    .into()
}

fn fork_ts_checker_webpack_plugin() -> String {
    r###"new ForkTsCheckerWebpackPlugin();
"###
    .into()
}

fn hot_module_replacement_plugin() -> String {
    r###"new webpack.HotModuleReplacementPlugin();
"###
    .into()
}

fn html_webpack_plugin() -> String {
    r###"new HtmlWebpackPlugin({
    title: 'APP TITLE',
    favicon: path.resolve(cwd, './src/images/logo.svg'),
    template: path.resolve(cwd, './src/index.html'),
    filename: 'index.html'
});
"###
    .into()
}

fn mini_css_extract_plugin() -> String {
    r###"new MiniCssExtractPlugin({
    filename: 'styles/[name].[chunkhash].css',
    chunkFilename: 'styles/[name].[chunkhash].chunk.css',
});
"###
    .into()
}

fn copy_webpack_plugin(aliases: Aliases) -> String {
    format!(
        r###"new CopyWebpackPlugin({{
  patterns: [
    {{
      from: '{}',
      to: 'assets',
      globOptions: {{
        ignore: ['*.DS_Store'],
      }},
      noErrorOnMissing: true,
    }},
  ],
}});
"###,
        aliases.public
    )
}

fn react_refresh_webpack_plugin() -> String {
    r###"new ReactRefreshWebpackPlugin();
"###
    .into()
}
