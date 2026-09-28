# Ruff 逐文件原生设置观察（2026-09-25）

Rust runtime 在同一工具、源码、配置、总截止时间和私有证据目录下执行 `ruff check --show-files`、`ruff check --show-settings` 与 JSON 扫描。当前设置解析只验收固定 Ruff 0.16.8；未知版本或设置输出损坏使该文件 `incomplete`，不把空报告解释为规则已执行。公开反馈 0.8、本地报告 0.7 对完整文件展示设置输出 SHA-256、候选映射 F401/E501 中全局启用的规则、是否存在逐文件忽略，以及固定 `coverage_proven=false`。原始设置输出仍留私有证据，不直接写入智能体对话。

原生扫描若报告 F401/E501，但同轮设置称该规则未全局启用，则保留诊断并标记 `rule_settings_report_mismatch`。逐文件忽略可使全局启用的规则在该文件无诊断；报告对此显示 `per_file_ignores_present=true`，不得据此签发规则覆盖。`noqa` 等源码内抑制尚未核验，所以即使无逐文件忽略，设置观察也不是完整覆盖证明。同步器对 0.7 报告严格检查设置结构；缺失设置、伪称覆盖已证明、未完成文件虚报设置均拒绝导入。旧 0.4/0.5/0.6 报告仍只按原有局部契约消费，不倒填设置身份。

验收包括适配器解析正反例、伪造“未启用却报 finding”的反例、同步器缺设置/伪称覆盖反例、真实 Ruff E501/F401、逐文件忽略及已有复检链。整体 Rust 工作区与 OpenSpec 验证见变更 `verification.md`。受批准的 required rule 集合、`noqa`/其它原生 suppression 差异、工具与策略可信来源、所有目标及类别覆盖和项目门禁仍待实现。

源码注释抑制的后续原生对照见 [Ruff 抑制观察](ruff-native-suppression-observation.md)；本文件记录设置观察阶段的验收边界。
