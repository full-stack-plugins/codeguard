# Ruff 候选规则映射与白名单观察身份（2026-09-25）

`rulepacks/ruff_lint_preview_v1.json` 是版本化的 **候选规则映射**，只对应已核验的 Ruff 0.16.8 `F401`、`E501` 原生规则。它保存来源、许可、精确工具版本、原生规则 ID、CodeGuard 规则 ID 与原始规则链接；严格解析拒绝通配、重复、错误来源、无许可或自称批准。它不配置 Ruff 的启用规则，也不代表受保护策略批准。其他原生规则及不兼容工具版本仍保留诊断，但映射身份为空。

公开 Python lint 反馈为 0.7，本地报告为 0.6。完整文件中的已映射 finding 记录规则包原始字节 SHA-256 和 `candidate_unapproved`。同步器重算内置规则包摘要并核验映射；篡改摘要的 0.6 报告不能导入。`rules whitelist propose` 0.3 只从最新已同步、源码和配置未变的报告展示本地观察身份，仍列出 `approved_rulepack_identity`、适配器摘要、人工误报裁定和可信策略修订为缺失证据。候选恒为 null，门禁恒为 none，退出码恒为 3。

验证：`ruff_rulepack_contract` 3 项、`work_sync_contract` 普通测试、`lint_python_cli` 普通及 3 项固定 Ruff 原生测试、`whitelist_propose_contract` 普通及 1 项固定 Ruff 原生测试通过。完整工作区与 OpenSpec 验证结果以变更 `verification.md` 为准。规则包批准、原生配置覆盖证明、可信签名/策略来源、真实白名单门禁仍待实现；OpenSpec 4.3/4.8/4.9 不勾选。
