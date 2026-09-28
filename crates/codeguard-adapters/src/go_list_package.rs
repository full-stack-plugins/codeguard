//! Go list 的已知包字段；未知原生字段保留兼容，重复已知字段拒绝。
use serde::Deserialize;
use serde_json::Value;
/// Go 1.23.4 本地包清单的解析载体。
#[derive(Deserialize)]
pub(crate) struct GoListPackage {
    #[serde(rename = "Dir")]
    pub directory: String,
    #[serde(rename = "ImportPath")]
    pub import_path: String,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "GoFiles", default)]
    pub go_files: Vec<String>,
    #[serde(rename = "CgoFiles", default)]
    pub cgo_files: Vec<String>,
    #[serde(rename = "TestGoFiles", default)]
    pub test_go_files: Vec<String>,
    #[serde(rename = "XTestGoFiles", default)]
    pub external_test_files: Vec<String>,
    #[serde(rename = "IgnoredGoFiles", default)]
    pub ignored_files: Vec<String>,
    #[serde(rename = "Incomplete", default)]
    pub incomplete: bool,
    #[serde(rename = "DepOnly", default)]
    pub dependency_only: bool,
    #[serde(rename = "Goroot", default)]
    pub go_root: bool,
    #[serde(rename = "Error", default)]
    pub error: Option<Value>,
    #[serde(rename = "DepsErrors", default)]
    pub dependency_errors: Vec<Value>,
}
