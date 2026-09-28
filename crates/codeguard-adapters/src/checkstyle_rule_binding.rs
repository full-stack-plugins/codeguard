/// 原配置到原生 source 的局部映射；不是批准规则、执行覆盖或白名单权威。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckstyleRuleBinding {
    /// 原生报告应精确匹配的完整 source。
    pub native_source: String,
    /// 原配置声明的内置检查类。
    pub checker_class: String,
    /// 用于对话解释的中文规则摘要。
    pub summary: &'static str,
    /// 原检查器官方规则说明引用。
    pub rule_reference: &'static str,
    /// 可读修复方向；不授权修改规则、源码抑制或自动白名单。
    pub repair_steps: Vec<&'static str>,
}
