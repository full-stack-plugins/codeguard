# 点前缀普通发现范围汇总

`codeguard detect` 的 DiscoveryReport 0.3.0 新增 `scope_summary`。普通发现仍按既有点前缀策略跳过路径根，不递归展开或逐文件报警；配置发现允许 `.ruff.toml` 与 `.pre-commit-config.yaml` 普通文件。摘要分别记录跳过的点前缀根数、实际观察到的配置例外文件数，并明确 `git_safety_status=not_evaluated`。这三个值不能用作“入库安全已检查”的证据。

目标 CLI 测试先因旧协议缺少范围汇总失败，后验证 `.hidden.py`、`.hidden_dir`、`.env` 不进入普通源码报告或逐路径警告，配置例外仍可观察；`detect_cli` 14 项通过。此切片没有实现 Git 安全例外，也没有实现 `codeguard/` 自有产物的精确排除，因此 OpenSpec 4.6、9.2 仍未完成。
