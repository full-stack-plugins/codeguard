use crate::{ApprovalTrustKey, ApprovalVerificationContext};
use std::{path::Path, time::Instant};
/// 受保护宿主提供的语法任务复检输入；密钥、上下文及原样本必须独立于项目选取。
/// 不暴露为 CLI 公钥参数；宿主须保证可信来源并固定当前策略、基线及防回滚序号。
#[derive(Clone, Copy)]
pub struct SyntaxTaskResolutionRequest<'a> {
    /// 已初始化的实际工作区。
    pub root: &'a Path,
    /// 原稳定任务身份。
    pub task_id: &'a str,
    /// 宿主选择且由批准策略固定摘要的原生语法工具。
    pub tool: &'a Path,
    /// 原始源码反例；只经 stdin 使用，不写入公开历史。
    pub original_source: &'a [u8],
    /// 原始限定任务策略字节。
    pub policy_bytes: &'a [u8],
    /// 绑定策略字节的签名信封。
    pub envelope_bytes: &'a [u8],
    /// 宿主独立固定的验签密钥。
    pub trust: &'a ApprovalTrustKey,
    /// 宿主独立固定的时间、工作区、基线与策略上下文。
    pub context: &'a ApprovalVerificationContext<'a>,
    /// 原始请求共用截止时间，不因两次检查重置。
    pub deadline: Instant,
    /// 可选已领取的 owner/token；没有时申请并释放自己的短期租约。
    pub borrowed_lease: Option<(&'a str, &'a str)>,
}
