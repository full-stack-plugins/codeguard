//! 批准的宿主上下文，不接受项目可写参数自行授权。

/// 受保护宿主在扫描前固定的目标、基线、时间和防回滚约束。
#[derive(Clone, Copy, Debug)]
pub struct ApprovalVerificationContext<'a> {
    /// 当前扫描工作区身份。
    pub workspace_id: &'a str,
    /// 预先选择的策略修订。
    pub policy_revision: &'a str,
    /// 受保护代码基线提交，支持 Git SHA-1/SHA-256。
    pub baseline_commit: &'a str,
    /// 可信时钟，缺失不能核验批准。
    pub now_unix: Option<u64>,
    /// 宿主固定的最小策略序号，禁止加载旧签名回滚策略。
    pub minimum_sequence: u64,
    /// 单条签发允许的最长有效期，秒。
    pub max_lifetime_seconds: u64,
}
