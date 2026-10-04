# 误报调查指引保留首次观察来源

对应 OpenSpec `introduce-rust-codeguard-cli` 的 remediation-workflow 场景与 9.7、9.10、14.12。这里只验收调查指引，不完成完整白名单、宿主审批或发行。

原反例在原生工具下未检出诊断，只能说明需要调查首次观察和本次对照的差异，不能直接确认 grammar 缺陷。原生首次任务没有 grammar；简报应检查输入、工具与环境差异。WASM 首次任务应检查语法资产、语言版本和原生对照差异。首次报告缺失或身份不符时保留决策需求，不推断原因。

## 行为与回归证据

Kotlin、Swift 受控反例测试读取 next、task show JSON 和人类可读输出，验证相同调查步骤；查询前后检查器调用记录、追加事件及租约字节不变。Erlang WASM 首次反例保留语法资产对照指引，明确不直接认定 grammar 缺陷。

修改前 Kotlin 测试因原生首次任务仍提示“调查 grammar 误报”而失败；修改后默认受影响反例测试 2 passed。受影响 WASM 五目标 48 passed / 0 failed / 4 ignored。忽略的真实工具场景不计入本批通过数；本批受控工具只证明来源分流和只读不变量，不证明生产误报率。

```bash
cargo test --locked --offline -p codeguard-cli --features wasm-precheck \
  --test erlang_task_resolution_service --test kotlin_task_resolution_service \
  --test swift_task_resolution_service --test task_resolution_service \
  --test task_lease_contract
```

报告协议和关闭规则不变：调查仍为 verification_required，不能从任务文字得到白名单批准、可信关闭或完整交付。完整纠错体系和实际宿主链路保持未完成。

最终静态检查：默认与 WASM 的 workspace/all-targets Clippy `-D warnings`、fmt、分层、OpenSpec strict 与 diff 检查均通过。本批未重跑完整默认工作区测试，不借用此前提交的全套结果。
