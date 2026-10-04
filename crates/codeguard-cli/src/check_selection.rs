//! 按固定语言注册表约束检查入口的执行范围。

/// 检查选择；对应 CodeGuard 统一入口契约，无 Java 对等对象。
#[derive(Clone, Eq, PartialEq)]
pub(crate) enum CheckSelection {
    All,
    Java,
    Language(String),
}

impl CheckSelection {
    /// 解析规范选择；参数为入口字符串，返回注册表已登记选择或错参原因。
    pub(crate) fn parse(value: Option<&str>) -> Result<Self, String> {
        match value {
            Some("all") => Ok(Self::All),
            Some("java") => Ok(Self::Java),
            Some(id)
                if codeguard_adapters::legacy_registry()?
                    .languages
                    .iter()
                    .any(|l| l.id == id) =>
            {
                Ok(Self::Language(id.to_owned()))
            }
            _ => Err("check 需要 all 或注册表规范语言 ID".into()),
        }
    }
    /// 返回报告使用的规范选择 ID。
    pub(crate) fn as_str(&self) -> &str {
        match self {
            Self::All => "all",
            Self::Java => "java",
            Self::Language(id) => id,
        }
    }
    /// 判断参数语言是否在请求范围内。
    pub(crate) fn includes(&self, language: &str) -> bool {
        self.language().is_none_or(|selected| selected == language)
    }
    /// 判断请求是否适用共享 npm/ESLint 构建根。
    pub(crate) fn includes_node(&self) -> bool {
        self.includes("javascript") || self.includes("typescript")
    }
    /// 判断参数检查器是否属于选定语言或适用共享生态。
    pub(crate) fn includes_checker(&self, checker: &str) -> bool {
        self == &Self::All
            || (self.includes_node() && checker.starts_with("node."))
            || checker
                .split('.')
                .next()
                .is_some_and(|id| self.includes(id))
    }
    /// 返回局部语法范围；全项目返回 None。
    pub(crate) fn language(&self) -> Option<&str> {
        if self == &Self::All {
            None
        } else {
            Some(self.as_str())
        }
    }
}
