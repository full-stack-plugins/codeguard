# Codeguard CLI

本目录是统一 Rust Codeguard 的独立源码工作区。当前阶段实现版本输出、旧语言清单观察、[Maven 检查器只读配置探测](tests/acceptance/maven-config-discovery.md)、[Ruff 项目配置探测与单文件原生复测](tests/acceptance/ruff-config-discovery.md)、[Python Ruff 项目扫描与可展示反馈数据](tests/acceptance/python-lint-scan.md)、[运行报告的智能体反馈摘要](tests/acceptance/conversation-feedback-baseline.md)、门禁结果领域聚合，以及 [Unix 原生进程执行层原型](tests/acceptance/process-runtime-baseline.md) 和 [协议 schema](tests/acceptance/protocol-schemas-baseline.md)；[生成的能力矩阵](docs/CAPABILITIES.md)中全部六类别×五候选平台仍为 `gap`，不能用旧 `stable` 标签、配置声明或单个验证服务宣称新扫描能力。矩阵协议见 `schemas/capability-inventory.schema.json`，Ruff 原生报告边界见 `tests/acceptance/native-ruff-baseline.md`。

规范事实源暂存于相邻 `codeguard-plugin/openspec/changes/introduce-rust-codeguard-cli/`。`lint python` 已提供配置感知的原生 Ruff 局部检查与 human/JSON 反馈，但固定返回未完成；工具批准、完整质量策略和宿主接入完成前，不能用本二进制签发质量通过。

`rules whitelist list/explain --candidate FILE` 是显式候选文件的只读查询，成功只表示结构检查已完成，输出始终为权威未核验、门禁无影响；见 [验收切片](tests/acceptance/whitelist-candidate-inspection.md)。

`init [path] [--dry-run|--apply]` 已提供工作区静态预览、受控创建及根 AGENTS 受管区块；apply 当前返回 partial/退出 3，准备任务、画像刷新和可信质量策略仍待实现，见 [验收切片](tests/acceptance/init-workspace-preview.md)。

`plan <类别|check> <语言|all> [path]` 已提供[只读计划预览](tests/acceptance/plan-readonly-preview.md)；可信策略与工具锁缺失时列出候选和缺口并退出 3，不产生正式执行计划或质量认证。

`config validate|explain [path] [--policy-candidate FILE]` 已提供[只读配置观察](tests/acceptance/config-readonly-inspection.md)：识别旧 `codeguard.json` 和工具锁，显式候选可检验必需检查、精确排除与工具锁摘要。候选和本地工具锁相符仍不构成独立批准，命令固定退出 3，不形成有效白名单或质量门禁。

`lint java FILE` 已提供[单文件原生 Maven/P3C 观察](tests/acceptance/java-p3c-cli-native-local.md)：从显式指定的本机工具、JDK 和离线依赖仓执行声明的十个 P3C 规则集，向终端反馈结构化诊断；即使没有诊断也不宣称覆盖完整，固定退出 3。`--checker p3c` 与缺省行为相同。

`lint java FILE --checker javadoc --java-home ABS_PATH` 提供[JDK 21 原生 Javadoc 单文件诊断](tests/acceptance/java-javadoc-cli-native-local.md)。已知缺失注释规则显示局部发现；未知输出、工具故障及缺少类路径保留为未完成。命令固定退出 3，不签发项目质量结论。
`check java PATH --java-home ABS_PATH` 在静态确认 Maven Javadoc 配置时还会执行相同的[JDK 项目局部探针](tests/acceptance/check-java-javadoc-partial.md)并把原生诊断反馈到终端；缺配置不调度探针。此结果不归因于 Maven Javadoc 插件的实际生命周期执行，也不进入修复任务或质量门禁。

```bash
cargo test --workspace --offline
cargo run --offline --bin codeguard -- --version
cargo run --offline --bin codeguard -- capabilities java --format json
cargo run --offline --bin codeguard -- lint python . --format json
cargo run --offline -p codeguard-cli --example gen_capability_docs -- --check
cargo run --offline -p codeguard-cli --example validate_corpus -- --verify-ruff /opt/anaconda3/bin/ruff --verify-maven /opt/homebrew/bin/mvn
```
