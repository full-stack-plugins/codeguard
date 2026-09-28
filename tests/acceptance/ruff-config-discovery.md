# Ruff 项目配置探测与原生单文件复测（2026-09-24）

`codeguard detect` 现在对 Python 源文件沿目录层级只读观察 `.ruff.toml`、`ruff.toml` 与 `pyproject.toml`。按 Ruff [官方配置发现规则](https://docs.astral.sh/ruff/configuration/#config-file-discovery)，同目录的 `.ruff.toml` 优先于 `ruff.toml`，后者优先于含 `[tool.ruff]` 的 `pyproject.toml`；不含该表的 pyproject 会被跳过。解析使用 Rust `toml` 0.8.23，只有显式 lint 表或设置才返回 `configured`；仅格式化设置和未解析 `extend` 返回 `unknown`，坏专用 TOML 或错误 lint 结构返回 `invalid`。找不到项目级 Ruff 配置返回 `missing`，只表示**本次观察范围内未找到项目配置**，不推断 CI 命令或用户级配置不存在。`.pre-commit-config.yaml` 暂记 `unknown`，不靠字符串匹配虚构启用状态。

`detect_cli` 的 12 项集成测试覆盖 Ruff 显式 lint、仅格式化、无配置、坏 TOML、`extend`、嵌套配置，以及 Maven 既有边界。配置状态不表示工具已安装或规则已执行。多个目录有不同配置时保留不同 `build_root`，不以根配置代表嵌套源码。

`ruff_probe` 增加可选的项目配置绑定。绑定时先确认配置文件内容摘要、TOML lint 声明和原生优先级；原生调用不使用会[忽略所有配置文件的 `--isolated`](https://docs.astral.sh/ruff/configuration/)，而是先运行 `ruff check --show-files` 确认目标确实被选中，再用同一截止时间运行 `ruff check --no-cache --output-format json`。源码、配置、工具在执行前后复核；私有日志沿用安全运行时。未绑定配置的旧试运行仍使用 `--isolated`，只可作为隔离基线。

本机 `CODEGUARD_RUFF_BIN=/opt/anaconda3/bin/ruff cargo test -q -p codeguard-cli --test ruff_probe_contract -- --ignored` 的 5 项真实 Ruff 0.16.8 测试通过。其中 E501 是通过项目配置启用的非默认规则，被 `force-exclude` 排除的文件返回 `source_not_selected_by_ruff`，不能变成干净通过。默认测试拒绝仅格式化配置和更高优先级配置覆盖；新增假原生 JSON 反例证明跨文件诊断不被误归因，保留同文件发现同时将扫描标为未完成。此能力仍只处理**单文件局部证据**；未验证完整项目源集、全部生效规则、政策门禁或宿主对话，Python lint 能力仍为 gap，OpenSpec 5.10/7.2 不勾选。
