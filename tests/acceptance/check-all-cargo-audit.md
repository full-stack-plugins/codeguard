# Rust CVE 进入统一检查的局部验收

对应 OpenSpec `introduce-rust-codeguard-cli` 的 7.1 和误报白名单边界 4.8。`check all` 在发现 Rust 项目时单独调度 `rust.cve`，显式指定 `--cargo-audit-tool` 与 `--rustsec-db` 后复用 `cve rust` 的受控原生 cargo-audit 观察。未指定原生工具时，JSON 反馈 `cargo_audit_tool_not_selected`，Rust CVE 类别为 `native_incomplete`；不会把未运行的检查说成通过。

`check_all_cargo_audit` 两项普通集成测试使用受控原生输出，核对 `time 0.1.40` 的 `RUSTSEC-2020-0071` 保留在 JSON，对应脱敏发现保留在 SARIF，CVE 类别独立出现，数据库时效仍为 `unverified`，交付为 `incomplete`、退出 3。另用实际 CLI 输出经 Draft202012 验证 `check-feedback` 0.29：缺工具和有 advisory 的两种报告均符合 schema；有 advisory 时未变成有效白名单或交付通过。既有 cargo-audit 单独入口的真实原生正反例见 `cargo-audit-native-observation.md`。

当前只证明局部原生观察进入统一反馈。漏洞库可信来源及更新时效、受批准工具和规则、完整 Cargo workspace/features/targets、依赖图权威、稳定任务及原工具复检、可信白名单批准和最终门禁尚未接通。对话中可以发起精确误报调查，但本地候选或项目自写批准不能放行，OpenSpec 7.1/4.8 仍未完成。

最终目标用例 2 项通过，受影响的 Rust 文档 CLI 14 项通过、1 项需显式原生工具而跳过；完整离线工作区回归为 161 组、949 项通过、0 失败、97 项条件跳过。真实 cargo-audit 条件用例另显式执行 1 项并通过。全目标 Clippy `-D warnings`、格式、128 份 schema 静态校验及两份实际 JSON 反馈校验通过；OpenSpec 严格校验与插件差异空白检查通过。回归记录见插件 OpenSpec `verification.md`。
