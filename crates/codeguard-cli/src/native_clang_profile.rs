//! Clang原生探针的固定规则档案；显式探针不冒充项目实际生效配置。

/// 独立语法和文档警告档案；参数在应用层选择，不接受自由编译器参数。
#[derive(Clone, Copy)]
pub(crate) enum NativeClangProfile {
    /// 保留现有Wall/Extra/Pedantic语法观察，不改变旧协议。
    Syntax,
    /// 检查Clang认识的文档注释命令和描述，不证明全部API文档覆盖。
    Documentation,
    /// 同次原生扫描输出警告和函数文档AST，结构事实不冒充原警告或完整政策。
    DocumentationStructure,
    /// 公开单文件文档反馈同次AST的独立占位策略，不修改历史结构协议。
    DocumentationPlaceholders,
}
impl NativeClangProfile {
    /// 返回固定原生警告参数；无运行输入，返回静态字面argv列表。
    pub(crate) fn warning_flags(self) -> &'static [&'static str] {
        match self {
            Self::Syntax => &["-Wall", "-Wextra", "-Wpedantic"],
            Self::Documentation
            | Self::DocumentationStructure
            | Self::DocumentationPlaceholders => &["-Wdocumentation", "-Wdocumentation-pedantic"],
        }
    }
}
