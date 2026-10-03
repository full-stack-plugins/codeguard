# 限定语法任务关闭与复发重开

对应 OpenSpec：`introduce-rust-codeguard-cli`，`remediation-workflow` 的可信关闭、父链核对和复发场景；关联 9.7、9.10、9.11、14.10。本记录仅涵盖当前源码的 Zig 原生语法确认任务，不勾选完整父任务。

## 真实行为

受保护宿主通过 `verify_zig_task_resolution` 验签限定任务策略，固定工作区、任务、首次报告、原始反例、grammar、Zig 工具及宿主可执行制品摘要。宿主上下文另固定策略修订、基线、可信时间和防回滚序号。处理器不从项目文件加载验签公钥或自批准布尔值。

处理器复用既有租约，向同一 Zig 0.16.0 先送入原样本，再送入当前源码。批准到期会缩短原请求截止时间。原样本确有诊断、当前字节有变化且无诊断，才能记 `code_fixed`；原样本也原生合法，则要求误报调查。原生失败或并发修改源码保存待核验观察，不能关闭。

限定事件位于 `.codeguard/findings/<id>/events/lifecycle-*.json`；原样本和当前样本只有摘要、脱敏诊断位置进入 `.codeguard/state/resolution_evidence/`，不存原源码、原生消息、公钥私钥或租约 token。第一次事实保持不变。父链允许解决、待验证及复发，拒绝分叉、重复身份、缺父、循环或断开的历史；内容摘要只用于关联，不能作为签名或来源证明。

普通 `task verify` 在同一工具和原 grammar 下重新检出当前原生诊断时追加 `reopened`。受保护宿主随后重检不会创建重复事件；再次修复仍使用同一任务身份。`next` 不沿用旧关闭或旧疑似位置，实际复发保留当前原生修复指引。宿主收据仅对当次核验上下文有效；公开 CLI 本地查询不将历史记录升级为可信关闭，全部保持交付未评估。

## TDD 与反例

- `/tmp/codeguard-resolution-service-red.log`：缺少受保护宿主应用入口，测试编译失败。这只证明接口缺失。
- `/tmp/codeguard-resolution-next-red.log`：原生解决后 next 仍给旧准备指引，真实断言失败。
- `/tmp/codeguard-resolution-attempt-red.log`：解决处理器没有绑定既有 ready-to-verify 尝试，`awaiting_verification` 仍为 true，真实断言失败。
- `/tmp/codeguard-resolution-recurrence-red.log`：普通 task verify 再次检出原问题时没有追加重开事件，事件数仍 2，真实断言失败。
- `/tmp/codeguard-resolution-history-cause-red.log`：关闭事件被重算摘要并改为政策处置后，旧读者未拒绝；修正事件归因与证据结果的对应校验。
- `/tmp/codeguard-resolution-unavailable-red.log`：工具中途消失后的 not_run 证据被误判为损坏；修正未运行状态的空工具身份边界，仍保持待核验。
- 对应 GREEN：关闭→重复复检→公开 CLI 复发→宿主确认→再次修复，租约与尝试交接、原生误报反证、失败观察、源码并发变化、签名密钥错误/撤销/缺时钟/过期/回滚、重复字段、任务/范围/制品/grammar 身份错误、分叉历史及丢失证据。

测试中的签名密钥来源是夹具；受控 shell 工具验证编排与协议，不代表生产 Zig 正确性。单独运行真实已安装 Zig 的测试用于核对原样本、修复和复发；它仍不证明真实宿主信任来源、完整 lint 或全语言精度。

## 协议与复现

四份独立封闭协议：

- [限定策略 1.0](../../schemas/task-resolution-policy-v1.0.schema.json)
- [生命周期记录 0.1](../../schemas/task-lifecycle-record-v0.1.schema.json)
- [脱敏对照证据 0.1](../../schemas/task-resolution-evidence-v0.1.schema.json)
- [宿主收据 0.1](../../schemas/task-resolution-receipt-v0.1.schema.json)

```bash
cargo test -p codeguard-core --test task_resolution_contract --offline
cargo test -p codeguard-cli --features wasm-precheck \
  --test task_resolution_service --test syntax_task_verify \
  --test hook_syntax_tasks --test task_lease_contract --test task_verify_contract \
  --offline -- --test-threads=2
# 仅在已有 /opt/homebrew/bin/zig 0.16.0 的 macOS 上：
cargo test -p codeguard-cli --features wasm-precheck \
  --test task_resolution_service real_zig_original_and_fixed_source_resolution \
  --offline -- --ignored
```

验证结果另在本 change 的 verification 中记录，不把未运行或被忽略的原生测试计作通过。默认与 WASM 构建顺序执行，避免相互覆盖 CLI 二进制。

## 本轮实际结果

最终默认工作区：203 组、1122 passed、0 failed、105 ignored；core 契约 7/7。特性版 SDK 与语法复检 17 passed、0 failed、2 ignored，明确执行真实 Zig 对照 1/1。特性全目标 Clippy `-D warnings`、fmt、分层、OpenSpec strict 通过。176 份 schema 定义、29 份实际/文档示例与 6 类伪造反例验证通过。具体命令日志及 SHA-256 见 [当前变更验证记录](../../openspec/changes/introduce-rust-codeguard-cli/verification.md)。

## 未完成范围

默认插件/公开 CLI 的可信策略提供者及真实宿主自动关闭仍未接入。其它检查器、依赖或环境恢复、批准的目标删除/政策处置、白名单误报裁定、跨机器原样本/证据恢复、Windows、完整门禁、硬 I/O 截止与性能评测仍缺。历史证据缺失时要求核对并重检，不凭本地文件签发解决或 allow。本轮不发布 npm，也不修改插件锁。
