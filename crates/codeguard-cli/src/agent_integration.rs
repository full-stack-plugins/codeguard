/// 返回当前二进制真实接入能力；不替代专业领域的运行结果。
#[cfg(feature = "full-cli")]
pub(crate) fn tool() -> guard_install::Tool {
    guard_install::Tool::new(
        "codeguard",
        env!("CARGO_PKG_VERSION"),
        "修改代码后使用项目既有配置和原生检查器；读取完整结果及稳定修复任务，修复后复检。区分源码问题、环境阻塞和未完成检查；不得关闭规则或扩大白名单来伪造完成。原生命令的平台限制仍适用。",
        &[
            "detect",
            "init",
            "check",
            "lint",
            "task",
            "next",
            "status",
            "capabilities",
        ],
    )
}

/// 返回仅接入构建的能力，明确不包含专业运行时。
#[cfg(not(feature = "full-cli"))]
pub(crate) fn tool() -> guard_install::Tool {
    guard_install::Tool::new(
        "codeguard",
        env!("CARGO_PKG_VERSION"),
        "此构建仅提供智能体接入，未包含专业检查运行时。代码质量检查需要在受支持平台使用完整 CodeGuard 构建；不得把接入诊断视为代码通过。",
        &[],
    )
}
