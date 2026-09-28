/// 宿主预先固定的发行范围、时钟和防回滚约束，不授予网络许可。
#[derive(Clone, Copy, Debug)]
pub struct DistributionVerificationContext<'a> {
    /// 预期发行者。
    pub publisher_id: &'a str,
    /// 精确发行通道。
    pub channel: &'a str,
    /// 本轮选择的平台。
    pub platform: &'a str,
    /// 可信当前 Unix 秒；缺失无法绑定。
    pub now_unix: Option<u64>,
    /// 宿主固定最低序号，不接受项目自行回滚。
    pub minimum_sequence: u64,
    /// 签发最长有效期，秒。
    pub max_lifetime_seconds: u64,
}
