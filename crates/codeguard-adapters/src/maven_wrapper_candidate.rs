/// Maven Wrapper 的只读准备状态；候选不表示已执行 Maven。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MavenWrapperCandidate {
    /// 观察到的具体状态。
    pub state: &'static str,
    /// 从固定 distributionUrl 观察到的 Maven 版本。
    pub observed_version: Option<String>,
    /// 下一步核验或修复建议。
    pub next_action: &'static str,
}
