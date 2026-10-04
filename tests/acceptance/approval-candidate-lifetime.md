# 白名单候选签名期限局部验收

日期：2026-10-04。对应既有 rulepack-governance 签名批准场景、4.8/4.10。父任务保持未完成。

## 已复现缺口及修复

原签名绑定器核验签名载荷自身有效期，但未核对候选有效期是否跨出该窗口。通过真实 Ed25519 测试密钥签发快照，将候选到期时间从200扩大为201、快照允许期限从100扩大为101，签名载荷仍为100至200。在可信测试时钟150，原普通绑定及根历史候选错误返回 BoundToPinnedSnapshot；两个独立 RED 日志记录实际失败。

当前普通候选、修订链当前候选和每个历史候选在严格快照绑定前核对：

- 候选到期不晚于该跳签名批准到期。
- 候选完整有效期不超过该跳宿主固定的最大期限。

越界返回 approval_candidate_lifetime_outside_signature，不静默延长期限。非法候选保留已有严格解析/绑定拒绝语义。候选形成早于签发本身不是错误，合法较短期限及相等上限仍允许绑定。

## 实际验证与边界

四个受影响目标：快照15 passed、报告白名单12 passed、签名链6 passed、普通签名6 passed；均0 failed/0 ignored。新增普通用例包含两个互相独立的超限反例和两个合法边界；链用例对根、中间、当前三跳分别检查签名到期超限与宿主期限超限，共六个变体。原生 Git 临时仓库的基线绑定用例也实际执行。这些是真实密码学与 Git 契约测试，身份及审批来源仍是测试输入，不代表外部人工审批、真实原生检查或公开插件门禁验收。

默认/WASM全目标CLI Clippy -D warnings、fmt、crate分层和OpenSpec strict通过；未改schema、原生工具调用、npm包或插件锁。保护中的Erlang差分草稿字节不变，不属于此提交。

仍缺真实受保护审批发布与可信时钟/策略来源、各原生检查器完整身份、批准处置与真实交付门禁接线、全格式宿主及生命周期验收；签名绑定成功不自行放行。本轮不勾选完整白名单或纠错父任务。

## 日志身份

- `/tmp/codeguard-approval-expiry-red.log`：`aa8d4e3889da51447a5712aa9e927904c1fef3d695d11abf8ed569ff655ddc92`。
- `/tmp/codeguard-approval-expiry-ordinary-red.log`：`0b198ce2e577b6e06eba1efc572021d5bc2cd77b12c83e9d0fa71f2b7b5bd18a`。
- `/tmp/codeguard-approval-expiry-final.log`：`4e6b36cc1aa3577dcebfb6c623a5e9b65aa3ef4e2f9c00525bb9486d2dc1354a`。
- `/tmp/codeguard-approval-expiry-clippy.log`：`05863b150a2a493d00393bf3d85f30218d7cd0499fb554c117b5b133fcd215b3`。
- `/tmp/codeguard-approval-expiry-wasm-clippy.log`：`0d8700d190339d21c7aa58a2763c0c5f6b98be9ca6d9758d130f5d6537133add`。
- `/tmp/codeguard-approval-expiry-openspec.log`：`cf3ec12601576e4a3dd335699d867edc30029dd07ee9fdcd6396379524b1fa4b`。
- `/tmp/codeguard-approval-expiry-layering.log`：`280c626d05d03e0944fca548fce5a1e9039200ab2cdd1f3d68f64c12e236de36`。

## 全套回归发现的旧处置断言

2026-10-05 完整默认套件揭露 signed_disposition_binding_contract 的旧正例仍期望把越界候选裁剪后接受。修正正例为合法期限，并新增越界拒绝断言；身份漂移/撤销/过期负例也使用合法候选，避免被期限拒绝短路。生产安全规则不放宽。该目标4 passed；完整重跑单独验收，见[契约回归记录](contract-regression-drift.md)。
