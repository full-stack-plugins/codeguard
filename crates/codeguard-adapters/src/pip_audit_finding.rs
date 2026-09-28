//! pip-audit 原生 advisory 观察；不得从别名或修复版本推断策略批准。

/// 原生报告中一个组件对应的一项已知漏洞。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PipAuditFinding {
    /// 原生漏洞标识，可能为 PYSEC、GHSA 或其它来源。
    pub advisory_id: String,
    /// 原生解析组件名。
    pub package_name: String,
    /// 原生解析版本。
    pub package_version: String,
    /// 原生提供的全部别名，保留来源但不合并问题身份。
    pub aliases: Vec<String>,
    /// 别名中语法有效的 CVE 标识。
    pub cve_aliases: Vec<String>,
    /// 原生建议版本，尚未验证可升级性或兼容性。
    pub fix_versions: Vec<String>,
}
