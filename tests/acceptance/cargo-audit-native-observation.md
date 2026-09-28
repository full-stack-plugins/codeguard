# Rust 原生 CVE 局部观察

OpenSpec `introduce-rust-codeguard-cli` 7.1 的局部实现。`codeguard cve rust [path] --cargo-audit-tool ABS_PATH --db ABS_PATH --format json` 使用共享 Rust 进程运行时调用原生 `cargo-audit audit --no-fetch --no-yanked --json --db ... --file Cargo.lock`；没有 Python 检测逻辑，也不下载或更改项目依赖。

TDD：adapter 合同测试先因缺 `parse_cargo_audit_json` 无法编译，公开命令测试先因入口缺失失败。实现后，严格原生 JSON 解析覆盖漏洞、零漏洞、ignore、重复键、错误退出码、错误组件；本轮 Cargo.lock 字节比对拒绝解析版本或来源错配。CLI 假原生工具样本验证公开 finding 保留 advisory/解析版本，未知数据库不授予交付通过。显式提供本机 `cargo-audit` 与离线 RustSec 数据库后，真实含 `time 0.1.40` 的锁文件检出 `RUSTSEC-2020-0071`；真实当前工作区锁文件零漏洞。两种情况均固定退出 3、`database_freshness=unverified`、`delivery_decision=not_evaluated`。

本机 cargo-audit JSON 对数据库报告 `last-commit=null`、`last-updated=null`；即便有值，当前 CLI 也尚未从可信来源核验其签名、提交和时效。公开报告只展示依赖来源 SHA-256，私有来源原文不进入对话。`schemas/rust-cve-local-observation.schema.json` 的 Draft202012 静态合法性及真实零漏洞输出已验证；篡改为 allow、coverage_proven=true 或 database_freshness=verified 的报告均被 schema 拒绝。

普通用例：`cargo test --offline -p codeguard-adapters --test cargo_audit_contract -p codeguard-cli --test cargo_audit_cli`。真实漏洞用例：设置已有 `CODEGUARD_CARGO_AUDIT_BIN`、`CODEGUARD_CARGO_AUDIT_DB` 后运行 `cargo test --offline -p codeguard-cli --test cargo_audit_cli -- --ignored`。

未完成：数据库可信 pin/时效、原生工具批准、完整锁图/工作区/目标组合、安全策略与严重度阈值、稳定任务及复检、`check all`/宿主接线、白名单正式门禁；`--no-yanked` 表示 yanked 包治理仍需独立检查。7.1 不勾选。
