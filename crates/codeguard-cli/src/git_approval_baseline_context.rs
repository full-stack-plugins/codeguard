//! 签名批准基线关系的受保护宿主输入。

use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

/// 宿主固定的只读仓库、Git 工具及共同截止时间。
///
/// 本类型不认证路径或工具来源，宿主必须使用隔离的可信仓库和已批准 Git。
pub struct GitApprovalBaselineContext<'a> {
    /// 宿主选择的仓库根绝对路径。
    pub root: &'a Path,
    /// 已批准 Git 二进制绝对路径；不会从项目 PATH 搜索。
    pub git_tool: &'a Path,
    /// 当前请求共同截止时间，各跳不得重新计时。
    pub deadline: Instant,
    /// 上层请求的取消标记，各跳共享且不得重置。
    pub cancelled: &'a AtomicBool,
}
