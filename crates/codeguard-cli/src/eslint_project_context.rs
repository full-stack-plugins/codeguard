use serde::Deserialize;
/// 调用者显式选择的子项目根、原配置和工作目录；不代表策略批准。
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct EslintProjectContext {
    pub(crate) root: String,
    pub(crate) config: String,
    pub(crate) cwd: String,
}
