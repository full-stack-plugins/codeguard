# Python WASM 语法候选兜底：局部验收

对应 OpenSpec `introduce-rust-codeguard-cli` 的 14.1、14.6、14.7、14.9 局部进展。此记录只适用于源码以 `wasm-precheck` 特性构建的 CLI；公开 npm `0.1.2` 未包含此能力。

`lint python` 先执行原有 Ruff 路径。只有选中文件的原生反馈为 `incomplete`、无原生 finding，且原因是 Ruff 工具缺失或项目未明确声明 Ruff 配置时，才由固定 Python grammar 和有界 Rust worker 补充疑似语法观察。无效 Ruff 配置保留原生阻塞；Ruff 已执行时不会用 WASM 覆盖诊断。最多处理八个文件、每文件 1 MiB、总计 128 个疑似位置，并沿用命令截止时间。超范围文件明确列在 `unavailable`。

JSON 使用 [Python 反馈 0.14.0](../../schemas/python-lint-feedback-v0.14.schema.json)。示意字段如下，`native.reason` 保留实际原因，不统一伪称工具缺失：

```json
{
  "schema_version": "0.14.0",
  "command_status": "incomplete",
  "native": { "status": "incomplete", "reason": "project_ruff_config_not_found" },
  "setup": { "requirement": "required", "reason": "native_confirmation_needed", "task_id": null },
  "syntax_precheck": {
    "backend": "bundled_tree_sitter_wasm_candidate",
    "status": "incomplete",
    "reason": "grammar_version_unqualified",
    "grammar_qualified": false,
    "selected_files": 1,
    "checked_files": 1,
    "observations": [{ "path": "broken.py", "classification": "suspected" }]
  },
  "delivery_decision": "not_evaluated",
  "exit_code": 3
}
```

此片段展示关键字段，完整报告另含源码/grammar SHA-256、位置、配置和同步状态。候选有位置不等于已证实源码违规；零位置也不等于 clean。`task_id: null` 表示尚未接通稳定 Python 原生确认任务，不能冒充已创建的任务。

验证命令：

```bash
CARGO_NET_OFFLINE=true cargo test --locked -p codeguard-cli --features wasm-precheck --test python_syntax_fallback_candidate
```

五个端到端反例覆盖配置的 Ruff 缺失、项目未声明 Ruff、有效语法、无效 Ruff 配置和超大兄弟文件；均保持退出码 3、`delivery_decision=not_evaluated`。真实 Ruff 优先路径另由 `lint_python_cli` 的忽略式本机用例核对。报告经 0.14.0 JSON Schema 校验。

仍未完成：Python grammar 的版本/方言及语料验收、项目级多模块调度、稳定原生确认任务、能力匹配的复检关闭、公开发行和真实宿主对话渲染。因此相关 OpenSpec 总任务保持未勾选。
