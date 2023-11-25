use crate::{
    libs::{
        project_aliases::{Aliases, ProjectAliases},
        project_config::{EnvType, ProjectConfig},
    },
    templates::Framework,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{error::Error, fs, io::Write, ops::Add, string::String};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebpackConfig {
    pub project_config: ProjectConfig,
    pub config: Value,
    pub constants: Vec<String>,
    pub plugins: Vec<String>,
    pub rules: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WebpackFrameworkConfig {
    pub constants: Vec<String>,
    pub plugins: Vec<String>,
    pub rules: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Env {
    name: &'static str,
    file: &'static str,
    config: WebpackConfig,
}

impl Env {
    fn get_js_config(&mut self) -> Result<Vec<String>, Box<dyn Error>> {
        let mut imports: Vec<String> = Vec::new();

        for constant in &self.config.constants {
            imports.push(format!("const {}", constant));
        }
        let rules: Vec<String> = self.config.rules.join(",\n").split("\n").map(|s| s.to_string()).collect();
        let plugins: Vec<String> = self.config.plugins.join(",\n").split("\n").map(|s| s.to_string()).collect();
        let js_object: Vec<String> = json_to_js_object(
            &self.config.config,
            &vec![
                InsertLines {
                    lines: rules,
                    into: "\"rules\": [%s],",
                },
                InsertLines {
                    lines: plugins,
                    into: "\"plugins\": [%s],",
                },
            ],
        );

        Ok(([imports, vec!["".into()], js_object]).concat())
    }
}

struct InsertLines {
    lines: Vec<String>,
    into: &'static str,
}

const INDENT_SIZE: usize = 2;
const DEV: &str = "Development";
const PROD: &str = "Production";
const CONFIG_DEV: &str = "webpack.config.dev.js";
const CONFIG_PROD: &str = "webpack.config.prod.js";
const IS_DEV: &str = "isDev = true;";
const IS_PROD: &str = "isDev = false;";
const PROCESS_CWD_CONST: &str = "cwd = process.cwd();";
const PATH_CONST: &str = "path = require('path');";
const WEBPACK_CONST: &str = "webpack = require('webpack');";
const FORK_TS_CHECKER_WEBPACK_PLUGIN_CONST: &str = "ForkTsCheckerWebpackPlugin = require('fork-ts-checker-webpack-plugin');";
const HTML_WEBPACK_PLUGIN_CONST: &str = "HtmlWebpackPlugin = require('html-webpack-plugin');";
const MINI_CSS_EXTRACT_PLUGIN_CONST: &str = "MiniCssExtractPlugin = require('mini-css-extract-plugin');";
const COPY_WEBPACK_PLUGIN_CONST: &str = "CopyWebpackPlugin = require('copy-webpack-plugin');";

pub fn get_config_dev(is_abs_path: bool, project_config: &ProjectConfig) -> WebpackConfig {
    let project_aliases = project_aliases(is_abs_path);
    let webpack_framework_config = Framework::get_webpack_config(&project_config.app.framework);
    WebpackConfig {
        project_config: project_config.clone(),
        config: config_dev(&project_aliases, &project_config),
        constants: vec![
            webpack_framework_config.constants,
            vec![
                PATH_CONST.into(),
                WEBPACK_CONST.into(),
                FORK_TS_CHECKER_WEBPACK_PLUGIN_CONST.into(),
                COPY_WEBPACK_PLUGIN_CONST.into(),
                HTML_WEBPACK_PLUGIN_CONST.into(),
                PROCESS_CWD_CONST.into(),
                IS_DEV.into(),
            ],
        ]
        .concat(),
        plugins: vec![
            vec![
                fork_ts_checker_webpack_plugin(),
                copy_webpack_plugin(&project_aliases),
                html_webpack_plugin(),
                hot_module_replacement_plugin(),
            ],
            webpack_framework_config.plugins,
        ]
        .concat(),
        rules: vec![vec![tsx_rule(), style_rule(), images_rule(), inline_rule()], webpack_framework_config.rules].concat(),
    }
}

pub fn get_config_prod(is_abs_path: bool, project_config: &ProjectConfig) -> WebpackConfig {
    let project_aliases = project_aliases(is_abs_path);
    let webpack_framework_config = Framework::get_webpack_config(&project_config.app.framework);
    WebpackConfig {
        project_config: project_config.clone(),
        config: config_prod(&project_aliases, &project_config),
        constants: vec![
            webpack_framework_config.constants,
            vec![
                PATH_CONST.into(),
                WEBPACK_CONST.into(),
                FORK_TS_CHECKER_WEBPACK_PLUGIN_CONST.into(),
                COPY_WEBPACK_PLUGIN_CONST.into(),
                HTML_WEBPACK_PLUGIN_CONST.into(),
                MINI_CSS_EXTRACT_PLUGIN_CONST.into(),
                PROCESS_CWD_CONST.into(),
                IS_PROD.into(),
            ],
        ]
        .concat(),
        plugins: vec![
            vec![
                fork_ts_checker_webpack_plugin(),
                copy_webpack_plugin(&project_aliases),
                html_webpack_plugin(),
                mini_css_extract_plugin(),
                hot_module_replacement_plugin(),
            ],
            webpack_framework_config.plugins,
        ]
        .concat(),
        rules: vec![vec![tsx_rule(), style_rule(), images_rule(), inline_rule()], webpack_framework_config.rules].concat(),
    }
}

pub fn show_config(env_type: EnvType) -> Result<(), Box<dyn Error>> {
    let mut env = get_env(&env_type);
    println!("\n✅ Webpack {} configuration:\n", env.name);
    for line in env.get_js_config()? {
        println!("{}", line);
    }
    Ok(())
}

pub fn export_config(env_type: EnvType) -> Result<(), Box<dyn Error>> {
    let mut env = get_env(&env_type);
    let mut file = fs::File::create(&env.file)?;
    for line in env.get_js_config()? {
        file.write_all(format!("{}\n", line).as_bytes())?;
    }
    let _ = ProjectConfig::get(&None).set_config(&env_type, env.file).save();
    println!("✅ Webpack {} configuration has been successfully exported to a file {}", env.name, env.file);
    Ok(())
}

fn get_env(env_type: &EnvType) -> Env {
    let project_config = ProjectConfig::get(&None);
    match env_type {
        EnvType::Dev => Env {
            name: DEV,
            file: CONFIG_DEV,
            config: get_config_dev(false, &project_config),
        },
        EnvType::Prod => Env {
            name: PROD,
            file: CONFIG_PROD,
            config: get_config_prod(false, &project_config),
        },
    }
}

fn json_to_js_object(json: &Value, insert_lines: &Vec<InsertLines>) -> Vec<String> {
    let json_str = serde_json::to_string_pretty(&json).unwrap();
    let mut lines: Vec<&str> = json_str.split("\n").collect();
    lines.remove(0);
    let insert_into_parts: Vec<Vec<&str>> = insert_lines.into_iter().map(|ins| ins.into.split("%s").collect()).collect();
    let mut new_lines: Vec<String> = Vec::new();
    new_lines.push("module.exports = {".into());
    for line in lines.into_iter() {
        let parent_indent = get_indent_size(line);
        let key = insert_into_parts.iter().position(|part| line.contains(part[0]));
        if key.is_some() {
            let k: usize = key.unwrap();
            new_lines.push(format_str(insert_into_parts[k][0], &Some(parent_indent)));
            for insert_line in &insert_lines[k].lines {
                new_lines.push(format_str(insert_line, &Some(parent_indent.add(INDENT_SIZE))));
            }
            new_lines.push(format_str(insert_into_parts[k][1], &Some(parent_indent)));
        } else {
            new_lines.push(format_str(line, &None));
        }
    }

    new_lines
}

fn format_str(line: &str, indent_size: &Option<usize>) -> String {
    let mut indent = " ".repeat(indent_size.unwrap_or_default());
    let re = Regex::new(r#"^(\s*)(")?(>{3})?((\w*)([^":]+)?)(")?(:\s)?(")?(>{3})?([^"]*)(")?(,?)$"#).unwrap();
    let Some(caps) = re.captures(line) else {
        return format!("{}{}", &indent, &line);
    };
    indent.push_str(&caps[1]);
    let key = &caps[4];
    let val = &caps[11];
    let comma = &caps[13];
    let mut quote_key = "";
    let mut quote_val = "";
    let mut colon = "";
    if caps.get(8).is_some() {
        colon = ": ";
    }
    if caps.get(2).is_some() && caps.get(7).is_some() && caps.get(3).is_none() && (caps.get(6).is_some() || caps.get(8).is_none()) {
        quote_key = "'";
    }
    if caps.get(8).is_some() && caps.get(9).is_some() && caps.get(12).is_some() && caps.get(10).is_none() {
        quote_val = "'";
    }
    format!(
        "{}{}{}{}{}{}{}{}{}",
        &indent, &quote_key, &key, &quote_key, &colon, &quote_val, &val, &quote_val, &comma
    )
}

fn get_indent_size(line: &str) -> usize {
    let re = Regex::new(r#"(^\s*)"#).unwrap();
    let Some(caps) = re.captures(line) else { return 0 };
    caps[1].len()
}

fn project_aliases(is_abs_path: bool) -> ProjectAliases {
    ProjectAliases {
        aliases: Aliases {
            src: "src".into(),
            build: "dist".into(),
            public: "public".into(),
            images: "src/images".into(),
            main: "src/main.ts".into(),
        },
        is_abs_path: is_abs_path,
    }
}

fn config_dev(project_aliases: &ProjectAliases, project_config: &ProjectConfig) -> Value {
    let aliases = project_aliases.to_owned().get();
    json!({
        "mode": "development",
        "entry": [&aliases.main],
        "output": {
          "path": &aliases.build,
          "publicPath": format!("{}://{}:{}/", project_config.dev.protocol, project_config.dev.host, project_config.dev.port),
          "filename": "js/[name].[contenthash].bundle.js",
          "assetModuleFilename": "assets/[hash][ext][query]",
        },
        "resolve": {
          "modules": [&aliases.src, "node_modules"],
          "extensions": [".tsx", ".ts", ".mjs", ".js", ".jsx", ".json", ".wasm", ".css"],
          "alias": project_aliases.to_owned().get_json(),
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
          "port": project_config.dev.port,
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

fn config_prod(project_aliases: &ProjectAliases, project_config: &ProjectConfig) -> Value {
    let aliases = project_aliases.to_owned().get();
    json!({
        "mode": "production",
        "entry": [&aliases.main],
        "output": {
          "path": &aliases.build,
          "publicPath": project_config.prod.public_path,
          "filename": "js/[name].[contenthash].bundle.js",
          "assetModuleFilename": "assets/[hash][ext][query]",
          "chunkFilename": "js/[name].[chunkhash].chunk.js",
          "clean": true,
        },
        "resolve": {
          "modules": [&aliases.src, "node_modules"],
          "extensions": [".tsx", ".ts", ".mjs", ".js", ".jsx", ".json", ".wasm", ".css"],
          "alias": project_aliases.to_owned().get_json(),
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
})"###
        .into()
}

fn style_rule() -> String {
    r###"new Object({
  test: /\.(sass|scss|css)$/,
  use: [
    { loader: isDev ? 'style-loader' : MiniCssExtractPlugin.loader },
    {
      loader: 'css-loader',
      options: {
        importLoaders: isDev ? 1 : 2,
        sourceMap: isDev,
        modules: isDev,
      },
    },
    { loader: 'postcss-loader', options: { sourceMap: isDev } },
    { loader: 'sass-loader', options: { sourceMap: isDev } },
  ],
})"###
        .into()
}

fn images_rule() -> String {
    r###"new Object({
  test: /\.(?:ico|gif|png|jpe?g)$/i,
  type: 'asset/resource',
  generator: {
    filename: 'assets/[hash][ext][query]',
  },
})"###
        .into()
}

fn inline_rule() -> String {
    r###"new Object({
  test: /\.(woff(2)?|eot|ttf|otf|svg|)$/i,
  type: 'asset/inline',
})"###
        .into()
}

fn fork_ts_checker_webpack_plugin() -> String {
    r###"new ForkTsCheckerWebpackPlugin()"###.into()
}

fn hot_module_replacement_plugin() -> String {
    r###"new webpack.HotModuleReplacementPlugin()"###.into()
}

fn html_webpack_plugin() -> String {
    r###"new HtmlWebpackPlugin({
  title: 'APP TITLE',
  favicon: path.resolve(cwd, './src/images/logo.svg'),
  template: path.resolve(cwd, './src/index.html'),
  filename: 'index.html'
})"###
        .into()
}

fn mini_css_extract_plugin() -> String {
    r###"new MiniCssExtractPlugin({
  filename: 'styles/[name].[chunkhash].css',
  chunkFilename: 'styles/[name].[chunkhash].chunk.css',
})"###
        .into()
}

fn copy_webpack_plugin(project_aliases: &ProjectAliases) -> String {
    format!(
        r###"new CopyWebpackPlugin({{
  patterns: [{{
    from: "{}",
    to: 'assets',
    globOptions: {{
      ignore: ['*.DS_Store'],
    }},
    noErrorOnMissing: true, 
  }}],
}})"###,
        project_aliases.to_owned().get().public
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn test_export_config(env_type: EnvType, file_name: &str) {
        let result = export_config(env_type);
        // Assert that the function returns a valid Result
        assert!(result.is_ok());
        // Check if the expected file was created
        let file_exists = fs::metadata(file_name).is_ok();
        assert!(file_exists);

        let generated_content = fs::read_to_string(file_name);
        let sample_content = fs::read_to_string(format!("examples/{}", file_name));
        if let (Ok(generated_content_str), Ok(sample_content_str)) = (generated_content, sample_content) {
            // Compare the generated content with the sample content
            assert_eq!(generated_content_str, sample_content_str);
        }
        // Clean up: remove the created file
        fs::remove_file(file_name).unwrap();
    }

    #[test]
    fn test_export_config_dev() {
        test_export_config(EnvType::Dev, CONFIG_DEV);
    }

    #[test]
    fn test_export_config_prod() {
        test_export_config(EnvType::Prod, CONFIG_PROD);
    }

    #[test]
    fn test_json_to_js_object() {
        let json: Value = json!({
          "key1": "value1",
          "key2": "value2",
          "key3": {},
        });
        let lines = vec!["line1: \"inserted1\",".to_string(), "line2: \"inserted2\",".to_string()];
        let into = "\"key3\": {%s},";
        let expected_result: Vec<String> = r#"module.exports = {
  key1: 'value1',
  key2: 'value2',
  key3: {
    line1: 'inserted1',
    line2: 'inserted2',
  },
}"#
        .split("\n")
        .into_iter()
        .map(|s| s.into())
        .collect();

        assert_eq!(json_to_js_object(&json, &vec![InsertLines { lines: lines, into: into }]), expected_result);
    }

    #[test]
    fn test_format_str() {
        let line = "\"key\": \"value\"";
        let indent = Some(2);
        let expected_result = "  key: 'value'";

        assert_eq!(format_str(line, &indent), expected_result);
    }

    #[test]
    fn test_get_indent_size() {
        let line = "  some text";
        let expected_result = 2;

        assert_eq!(get_indent_size(line), expected_result);
    }
}
