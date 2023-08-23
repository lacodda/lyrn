use super::helpers::ordered_map;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Package {
    pub name: String,
    pub version: String,
    pub description: String,
    pub main: String,
    pub scripts: HashMap<String, String>,
    pub keywords: Vec<String>,
    pub author: String,
    pub license: String,
    #[serde(serialize_with = "ordered_map")]
    pub dependencies: HashMap<String, String>,
    #[serde(serialize_with = "ordered_map")]
    pub dev_dependencies: HashMap<String, String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TsconfigCompilerOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_map: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub declaration: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module_resolution: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emit_decorator_metadata: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental_decorators: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub import_helpers: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module: Option<String>,
    pub lib: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jsx: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_lib_check: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_default_lib_check: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub es_module_interop: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_implicit_any: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolve_json_module: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub out_dir: Option<String>,
    pub paths: HashMap<String, Vec<String>>,
    pub exclude: Vec<String>,
}

impl TsconfigCompilerOptions {
    pub fn merge(self, common: &TsconfigCompilerOptions) -> TsconfigCompilerOptions {
        let mut compiler_options: TsconfigCompilerOptions = common.clone();
        compiler_options.lib.extend(self.lib);
        compiler_options.paths.extend(self.paths.into_iter());
        compiler_options.exclude.extend(self.exclude);
        TsconfigCompilerOptions {
            root_dir: self.root_dir.or(compiler_options.root_dir),
            source_map: self.source_map.or(compiler_options.source_map),
            declaration: self.declaration.or(compiler_options.declaration),
            module_resolution: self.module_resolution.or(compiler_options.module_resolution),
            emit_decorator_metadata: self.emit_decorator_metadata.or(compiler_options.emit_decorator_metadata),
            experimental_decorators: self.experimental_decorators.or(compiler_options.experimental_decorators),
            import_helpers: self.import_helpers.or(compiler_options.import_helpers),
            target: self.target.or(compiler_options.target),
            module: self.module.or(compiler_options.module),
            lib: compiler_options.lib,
            jsx: self.jsx.or(compiler_options.jsx),
            skip_lib_check: self.skip_lib_check.or(compiler_options.skip_lib_check),
            skip_default_lib_check: self.skip_default_lib_check.or(compiler_options.skip_default_lib_check),
            es_module_interop: self.es_module_interop.or(compiler_options.es_module_interop),
            no_implicit_any: self.no_implicit_any.or(compiler_options.no_implicit_any),
            resolve_json_module: self.resolve_json_module.or(compiler_options.resolve_json_module),
            base_url: self.base_url.or(compiler_options.base_url),
            out_dir: self.out_dir.or(compiler_options.out_dir),
            paths: compiler_options.paths,
            exclude: compiler_options.exclude,
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tsconfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compile_on_save: Option<bool>,
    pub compiler_options: TsconfigCompilerOptions,
}

impl Tsconfig {
    pub fn merge(self, common: &Tsconfig) -> Tsconfig {
        Tsconfig {
            compile_on_save: self.compile_on_save.or(common.compile_on_save),
            compiler_options: self.compiler_options.merge(&common.compiler_options),
        }
    }
}
