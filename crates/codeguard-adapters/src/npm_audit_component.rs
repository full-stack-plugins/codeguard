/// npm原生易受影响组件观察；范围不是已解析版本，source不是CVE编号。
#[derive(Debug)]
pub struct NpmAuditComponent {
    /// 原生组件名，仅供受保护内部身份核对；对话层必须另做脱敏。
    pub native_name: String,
    /// 原生严重度，不自行升级规则策略。
    pub severity: String,
    /// npm声明的直接依赖标记，尚未绑定本轮依赖图。
    pub is_direct: bool,
    /// 原生受影响版本范围，不能替代锁文件中的解析版本。
    pub affected_range: String,
    /// 原生节点相对位置，已检查路径归属语法。
    pub node_locations: Vec<String>,
    /// npm advisory source ID，不伪造CVE或GHSA别名。
    pub advisory_sources: Vec<u64>,
    /// 由其它原生组件造成的间接风险关系。
    pub via_components: Vec<String>,
}
