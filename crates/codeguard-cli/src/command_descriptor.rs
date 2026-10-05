use serde::Serialize;

/// 当前构建的静态命令描述；支持状态不授予检查或交付权威。
#[derive(Serialize)]
pub(crate) struct CommandDescriptor {
    /// C01–C36追踪编号；额外公开操作为空，不重复计数。
    pub tracking_id: Option<&'static str>,
    /// 可精确查询的命令前缀。
    pub command: &'static str,
    /// 已实现、部分实现、未实现或本构建不可用。
    pub support: &'static str,
    /// 当前构建是否存在该入口，不代表完整验收。
    pub executable: bool,
    /// 查询、规划、管理、检查或宿主协议类别。
    pub operation_kind: &'static str,
    /// 当前有限语法；planned项仅为目标前缀，不伪造可用参数。
    pub usage: &'static str,
    /// 实际能力范围与未完成边界。
    pub scope: &'static str,
    /// 当前入口的显式示例；绝对工具路径占位不构成自动安装或执行授权。
    pub examples: &'static [&'static str],
    /// 直接检查入口支持的规范语种；非检查操作为空。
    pub languages: &'static [&'static str],
}
