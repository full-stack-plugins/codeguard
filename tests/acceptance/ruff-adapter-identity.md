# Ruff 适配器本轮身份与白名单候选验收

对应 OpenSpec `introduce-rust-codeguard-cli` 4.8/4.9 的局部实现。原生 Ruff 0.16.8 的本轮 finding 保留文件、工具、配置、规则映射与发现指纹；本次增加 CodeGuard 可执行制品 SHA-256，作为 Rust 适配器**观察身份**。只有实际出现原生 finding、仍有检查预算且未取消时才读取二进制；失败或超时给 `null`，不填占位值。持久局部报告为 0.9、公开 lint 反馈为 0.12、只读误报提案预览为 0.4、Ruff 任务复检为 0.9；旧公开 schema 原字节留存。

初始真实 Ruff 测试先因 `observed_artifacts.adapter_sha256` 缺失失败。第一次实现把整个二进制哈希放在原生探测前，导致短预算及 SIGINT 两项回归失败；改为**原生 finding 返回后按需计算**，受影响普通组 12/4/15/10 项通过。实际 Ruff 任务复检八项、候选与纠错两项通过。另将本地报告的 adapter 摘要改成另一个合法 SHA-256，并同步改写消费标记的报告摘要，候选入口仍拒绝，原因 `adapter_binary_changed_since_scan`。

实际 CLI 输出已通过 Draft202012 校验：Python lint 0.12、候选预览 0.4、Ruff task verify 0.9（嵌入局部扫描 0.9）。提案仍有 `candidate=null`、`authority=unverified`、`gate_effect=none`；批准 rulepack、人工误报裁定和可信策略修订仍缺。可执行文件哈希不是独立签名或受保护发行证明，报告和本地同步标记仍可写；不能据此授予例外或项目交付通过。
