//! C/C++单文件文档原生请求，复用已校验的标准/工具参数，不接受自由argv。
use crate::syntax_lint_arguments::SyntaxLintArguments;

/// 已验证的C/C++文档请求；只复用语法参数结构，不复用语法规则档案或报告。
pub(crate) struct CFamilyCommentsArguments(pub(crate) SyntaxLintArguments);
impl CFamilyCommentsArguments {
    /// 解析完整语言后请求；参数包括规范语言、唯一源码、显式工具与标准，返回错参原因。
    pub(crate) fn parse(args: &[String]) -> Result<Self, String> {
        if !args
            .first()
            .is_some_and(|language| matches!(language.as_str(), "c" | "cpp"))
        {
            return Err("comments需要c或cpp及显式单文件上下文".into());
        }
        let request = SyntaxLintArguments::parse(args)
            .map_err(|reason| reason.replace("lint", "comments"))?;
        if request.clang_tool.is_none() || request.standard.is_none() {
            return Err("C/C++文档检查必须提供--clang-tool绝对路径与匹配的--standard".into());
        }
        if request.workspace.is_some() {
            return Err("C/C++文档单文件探针尚未支持工作台导入；不接受--workspace".into());
        }
        Ok(Self(request))
    }
}
