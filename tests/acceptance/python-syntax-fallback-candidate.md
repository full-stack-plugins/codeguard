# Python WASM 语法候选兜底：局部验收

对应 OpenSpec `introduce-rust-codeguard-cli` 的 14.1、14.6、14.7、14.9、14.10 局部进展。此记录只适用于源码以 `wasm-precheck` 特性构建的 CLI；公开 npm `0.1.2` 未包含此能力。

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

此片段展示未初始化工作区的关键字段，完整报告另含源码/grammar SHA-256、位置、配置和同步状态。候选有位置不等于已证实源码违规；零位置也不等于 clean。未初始化时 `task_id: null`，不能冒充已创建的任务。

已初始化工作区的单个已检查文件使用 [0.15.0 对话反馈](../../schemas/python-lint-feedback-v0.15.schema.json)，同步成功后 `setup.task_id` 指向真实稳定任务；同步失败时 `task_persistence` 保留具体原因且任务 ID 为空。脱敏位置、源码和固定 grammar 摘要写入 [0.1.0 本地确认报告](../../schemas/python-syntax-confirmation-observation-v0.1.schema.json)。任务指向适用原生语法能力复检，不把恢复节点认作原生违规。

验证命令：

```bash
CARGO_NET_OFFLINE=true cargo test --locked -p codeguard-cli --features wasm-precheck --test python_syntax_fallback_candidate
```

八个默认执行的端到端反例覆盖配置的 Ruff 缺失、项目未声明 Ruff、有效语法、无效 Ruff 配置、超大兄弟文件、任务稳定复扫、伪造 grammar/越界坐标/重复 JSON 键拒绝，以及持久化失败保留空任务 ID；均保持退出码 3、`delivery_decision=not_evaluated`。更改源码为有效语法后，再次 WASM 零恢复节点仍保持同一任务 `open`。额外使用本机 Ruff 0.16.8 运行 `task verify`，保存原生观察而不关闭候选任务；原生优先路径另由 `lint_python_cli` 的真实 Ruff 用例核对。实际生成的 0.15.0 反馈和 0.1.0 本地报告经 JSON Schema 校验，伪造同步成功状态被拒绝。默认二进制消费特性版保存的历史报告后，`work sync` 未出现失败报告。

仍未完成：Python grammar 的版本/方言及语料验收、多文件/项目级任务归并、能力匹配的原生复检关闭、公开发行和真实宿主对话渲染。因此相关 OpenSpec 总任务保持未勾选。
