/// Ruff 0.16.8 的七项 pydoclint 原生规则说明；只用于指引，不执行或批准规则。
pub struct RuffDocumentationRule {
    /// 脱敏规则摘要。
    pub summary: &'static str,
    /// 限定范围修复或调查步骤。
    pub step: &'static str,
    /// 是否须先调查规则约定和真实行为，不授权自动修复。
    pub investigation_required: bool,
}

impl RuffDocumentationRule {
    /// 按精确原生编号取得说明；未知编号返回 None，不能据 DOC 前缀推断已支持。
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        let (summary, step) = match code {
            "DOC102" => (
                "文档包含签名之外的参数",
                "核对实际签名与原配置约定，修正文档中不存在的参数；保留真实参数的用途与约束，不改变签名逃避检查",
            ),
            "DOC201" => (
                "文档缺少实际返回值说明",
                "核对实际返回行为，补齐 Returns 的返回含义、类型和适用条件；核对抽象方法等原生豁免，疑似误报走精确裁定；不要用空段落或删除用途说明逃避检查",
            ),
            "DOC202" => (
                "文档返回值说明与函数不一致",
                "核对实际返回行为和 API 契约，修正不适用的 Returns 描述；不要改变返回行为来迎合文档规则",
            ),
            "DOC402" => (
                "文档缺少实际生成值说明",
                "核对实际生成器行为，补齐 Yields 的值含义、类型及生成条件；不要删除用途说明或改变生成行为",
            ),
            "DOC403" => (
                "文档生成值说明与函数不一致",
                "核对实际生成行为和 API 契约，修正不适用的 Yields 描述；不要添加虚假 yield 来消除诊断",
            ),
            "DOC501" => (
                "文档遗漏直接抛出的异常",
                "核对实际 raise 分支，补齐 Raises 中异常类型及触发条件；隐式异常和调用链行为还须单独核验",
            ),
            "DOC502" => (
                "文档异常与原生直接抛出约定不一致",
                "先调查实际调用链、隐式异常和原项目约定；DOC502 只比对直接 raise，不得自动删除真实可能抛出的异常说明；误报走精确裁定",
            ),
            _ => return None,
        };
        Some(Self {
            summary,
            step,
            investigation_required: code == "DOC502",
        })
    }
}

/// 判断原生文档规则分类；参数为原规则编号，返回值不证明启用、语义覆盖或工具批准。
#[must_use]
pub fn is_ruff_documentation_rule(code: &str) -> bool {
    crate::ruff::is_ruff_pydocstyle_rule(code) || RuffDocumentationRule::from_code(code).is_some()
}
