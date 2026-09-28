# Ruff 局部报告的工具与配置观察身份（2026-09-25）

`lint python` 的公开反馈升至 0.6，持久局部报告升至 0.5。对原生证据完整的单个源码文件，报告附 `tool_sha256` 和 `config_sha256`：Rust 探针在调用前后核对工具字节与该源码实际选择的 Ruff 配置，整轮结束后再次核对；源码、配置或工具变化会把该文件改为 `incomplete`，清空这两个摘要，保留有效原生诊断供调查。缺配置、缺工具和其它未完成结果的摘要为 null。这些值是本地观察身份，`tool_approval=unverified`、`delivery_decision=not_evaluated` 不变，不能被白名单候选或门禁当作可信批准。

`work sync` 同时读取旧 0.4 报告和新 0.5 报告；新版本的完整文件若缺合法摘要、未完成文件却声称摘要，均拒绝导入。旧报告不被倒填身份。`next` 可核对两版复检报告，不把 schema 升级误判为任务已关闭。目标 `lint_python_cli` 8 项普通和 3 项固定 Ruff 0.16.8 原生测试、`work_sync_contract` 8 项普通测试及 `task_verify_contract` 的 5 项普通和 2 项真实 Ruff 测试通过；全工作区测试、Clippy、fmt 通过。

尚缺 Rust 适配器和批准 rulepack 的身份、可信工具锁/策略来源、本轮 finding 与候选的严格绑定及全格式门禁。`rules whitelist propose` 仍返回 `candidate=null`，OpenSpec 4.8/4.9/5.2/9.10 不因此勾选。
