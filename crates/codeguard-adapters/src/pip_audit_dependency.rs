//! pip-audit 原生 JSON 的已解析组件身份；归属仍须与项目输入核对。

/// 原生报告中一个已解析 Python 组件及版本。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PipAuditDependency {
    /// 原生规范化包名。
    pub name: String,
    /// 原生解析版本，不自动视作项目锁版本。
    pub version: String,
}
