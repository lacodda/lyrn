use json_value_merge::Merge;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{env, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebpackConfig {
    pub app_config: AppConfig,
    pub config: Value,
    pub plugins: Vec<String>,
    pub rules: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub protocol: String,
    pub host: String,
    pub port: i32,
    pub app_name: String,
    pub app_title: String,
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

pub fn get() -> WebpackConfig {
    WebpackConfig {
        app_config: app_config(),
        config: config_dev(),
        plugins: vec![
            fork_ts_checker_webpack_plugin(),
            copy_webpack_plugin(aliases()),
            html_webpack_plugin(),
            // mini_css_extract_plugin(),
            hot_module_replacement_plugin(),
            react_refresh_webpack_plugin(),
        ],
        rules: vec![tsx_rule(), style_rule(true), images_rule(), inline_rule()],
    }
}

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
    aliases_json.merge(json!({
        "@": aliases().src
    }));
    aliases_json
}

fn app_config() -> AppConfig {
    AppConfig {
        protocol: "http".into(),
        host: "localhost".into(),
        port: 8085,
        app_name: "react".into(),
        app_title: "React Boilerplate".into(),
    }
}

pub fn config_dev() -> Value {
    let config = app_config();
    json!({
        "mode": "development",
        "entry": [aliases().main],
        "output": {
          "path": aliases().build,
          "publicPath": format!("{}://{}:{}/", config.protocol, config.host, config.port),
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
          "port": config.port,
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
        r###"const MiniCssExtractPlugin = require('mini-css-extract-plugin');
let isDev = {};
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
    r###"const ForkTsCheckerWebpackPlugin = require('fork-ts-checker-webpack-plugin');
new ForkTsCheckerWebpackPlugin();
"###
    .into()
}

fn hot_module_replacement_plugin() -> String {
    r###"const webpack = require('webpack');
new webpack.HotModuleReplacementPlugin();
"###
    .into()
}

fn html_webpack_plugin() -> String {
    r###"const path = require('path');
const HtmlWebpackPlugin = require('html-webpack-plugin');
const cwd = process.cwd();
new HtmlWebpackPlugin({
    title: 'APP TITLE',
    favicon: path.resolve(cwd, './src/images/logo.svg'),
    template: path.resolve(cwd, './src/index.html'),
    filename: 'index.html'
});
"###
    .into()
}

fn _mini_css_extract_plugin() -> String {
    r###"const MiniCssExtractPlugin = require('mini-css-extract-plugin');
new MiniCssExtractPlugin({
    filename: 'styles/[name].[chunkhash].css',
    chunkFilename: 'styles/[name].[chunkhash].chunk.css',
});
"###
    .into()
}

fn copy_webpack_plugin(aliases: Aliases) -> String {
    format!(
        r###"const CopyWebpackPlugin = require('copy-webpack-plugin');
new CopyWebpackPlugin({{
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
    r###"const ReactRefreshWebpackPlugin = require('@pmmmwh/react-refresh-webpack-plugin');
new ReactRefreshWebpackPlugin();
"###
    .into()
}
