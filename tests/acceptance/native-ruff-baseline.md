# Ruff 原生解析基线（2026-09-24）

Ruff 0.16.8 是首个被复现的原生报告契约。`codeguard-adapters::parse_ruff_json` 只解析 `ruff check --output-format json` 的报告与原生退出码，保留规则 ID、路径、行列、严重度和有效诊断；配置/进程故障、无效 JSON、缺失字段、退出码与报告矛盾都标为 incomplete。完整 JSON 数组里若单条损坏，其余有效诊断仍保留。

本机实测命令：

```bash
CODEGUARD_RUFF_BIN=/opt/anaconda3/bin/ruff cargo test --offline -p codeguard-adapters --test ruff_native_replay -- --ignored
cargo test --offline -p codeguard-adapters --test ruff_contract
cargo run --offline -p codeguard-cli --example validate_corpus -- --verify-ruff /opt/anaconda3/bin/ruff --verify-maven /opt/homebrew/bin/mvn
```

真实 F401、干净文件、配置故障和缺文件 E902 各通过一项复现测试。E902 是 Ruff 原生 I/O 故障诊断，单独保存在环境诊断中，使本次结果未完成；它不转成源码违规。解析契约另覆盖退出 2、矛盾结果、坏报告、UNKNOWN 严重度和部分有效诊断。原始正反语料及可复现二进制摘要登记在 `tests/fixtures/corpus/oracle.json`，由语料验证器复核。

后续补充了 `codeguard-cli::ruff_probe` 单文件验证服务：要求调用者提供绝对 Ruff 路径、二进制 SHA-256、精确版本、普通源文件、共享截止时间与私有证据目录。它通过 Rust runtime 执行原生 `ruff --version` 和 `ruff check --isolated --output-format json`，执行前后复核二进制与源内容，解析结果并原子保存两份私有原始日志。`CODEGUARD_RUFF_BIN=/opt/anaconda3/bin/ruff cargo test --offline -p codeguard-cli --test ruff_probe_contract -- --ignored` 的 3 项真实测试通过：F401、干净文件、日志目标 symlink 导致明确未完成；2 项默认测试覆盖缺源码与二进制摘要不匹配的执行前拒绝。

这仍是**单文件局部证据**，不是已公开的 `lint python` 命令，更不能证明 Python 项目全量扫描。项目配置/规则权威、工具锁文件、扫描范围与依赖闭包、策略影响、脱敏公开报告、任务复检和交付认证尚未接入；Python lint 在能力矩阵仍为 `gap`，OpenSpec 5.4/5.5 与 7.2 不据此勾选。

项目配置探测及非隔离单文件复测的后续进展见 [Ruff 配置验收](ruff-config-discovery.md)。该进展验证了项目配置规则可被原生 Ruff 读取，但未将此试运行升级成项目全量 lint 能力。
