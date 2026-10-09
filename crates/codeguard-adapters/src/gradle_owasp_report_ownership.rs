use serde::{Deserialize, Serialize};

/// 一张原生OWASP报告的所属任务与输出身份；不代表依赖归属或漏洞库时效。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GradleOwaspReportOwnership {
    /// 原生所属项目路径。
    pub project_path: String,
    /// 显式执行的完整任务路径。
    pub task_path: String,
    /// 原生官方基类名称。
    pub implementation: String,
    /// 原任务用于报告的项目显示名。
    pub report_project_name: String,
    /// 私有受检副本内的唯一JSON输出相对路径。
    pub report_path: String,
}
