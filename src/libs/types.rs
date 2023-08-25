use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug)]
pub struct User {
    pub name: String,
    pub email: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Package {
    pub name: String,
    pub version: String,
    pub description: String,
    pub main: String,
    pub scripts: Value,
    pub keywords: Vec<String>,
    pub author: String,
    pub license: String,
    pub dependencies: Value,
    pub dev_dependencies: Value,
}