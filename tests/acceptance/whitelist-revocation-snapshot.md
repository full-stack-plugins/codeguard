# 误报白名单撤销快照局部验收

`approval-snapshot` 1.1 在同一策略修订中记录有效候选字节摘要与撤销 ID。`bind_candidate_to_snapshot` 先核对快照 pin 和结构，再解析候选；撤销 ID 优先返回 `Revoked`，不会因旧候选仍列在有效摘要表中而恢复匹配。旧 1.0 协议可读取，但不能携带撤销字段；1.1 必须显式提供撤销数组。

反例包括重复和通配撤销 ID、缺撤销数组、旧协议擅加撤销字段；均返回 `InvalidSnapshot`。受保护宿主尚未提供可信 pin、时钟、原生 finding 与门禁消费，此局部验收不等于白名单可实际放行或撤销线上批准。

本轮执行：`cargo test -p codeguard-cli --test approval_snapshot_contract --offline` 10/10；`cargo test --workspace --offline -q`、`cargo clippy --workspace --all-targets --offline -- -D warnings`、`cargo fmt --all --check` 均退出 0。
