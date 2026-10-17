//! Gradle原生生效模型的限定输入协议。
use crate::gradle_project_checker_model::GradleProjectCheckerModel;
use serde::{Deserialize, Serialize};
/// 单次原生配置观察的模型；执行、来源和质量资格由调用者独立核验。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GradleCheckerModel {
    /// 当前模型协议版本。
    pub schema_version: String,
    /// 固定模型报告类型。
    pub report_type: String,
    /// 原生Gradle声明的具体版本。
    pub gradle_version: String,
    /// composite build数量；当前模型边界不支持遗漏独立构建。
    pub included_build_count: u32,
    /// 根项目及每个子项目的独立检查器配置。
    pub projects: Vec<GradleProjectCheckerModel>,
}
