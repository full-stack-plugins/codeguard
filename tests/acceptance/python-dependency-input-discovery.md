# Python 依赖输入与 CVE 候选发现验收

OpenSpec 对应 `introduce-rust-codeguard-cli` 的 5.10、7.2，以及 `native-tool-adapters` 中 Python 依赖输入场景。本阶段只做 Rust 只读发现和反馈，没有运行 pip-audit、Python 包管理器或漏洞查询。

## 已验证行为

- `requirements.txt` 的简单 `name==version` 行被标为 `pinned_requirements_graph_unverified`；版本范围、引用、环境标记、URL、可编辑包、同一包的等价名称重复均归入未解析。行级精确版本不证明传递依赖图完整。
- `uv.lock` 或 `poetry.lock` 在同一构建根出现时，标为 `python_lock_model_unparsed`。锁的存在不证明内容已解析、来源可信或漏洞库匹配完成。根目录 requirements 与子项目锁分别归属各自构建根。
- 符合 [PyPA 命名规范](https://packaging.python.org/en/latest/specifications/pylock-toml/)的 `pylock.toml` 与 `pylock.ci.toml` 被识别为标准锁候选，标为 `python_standard_lock_model_unparsed`。与 `uv.lock` 同根并存时返回 `multiple_python_lock_inputs_unresolved`；仅有 uv/Poetry 锁时明确提示不能直接交给当前 [pip-audit `--locked` 入口](https://github.com/pypa/pip-audit/blob/main/README.md?plain=1)。
- 只有 Python 源文件而没有依赖输入时，标为 `python_dependency_input_not_found`。实现中对输入读取失败或摘要变化保留本轮发现不完整；该时序分支尚无独立故障注入验收。
- `detect` 显示逐根 `python.pip_audit` 配置 `unknown` 及具体下一步；`plan cve python` 不生成已可执行的任务，质量判断保持 `not_evaluated`；`check all` 的 CVE 类别为 `configuration_unresolved`，交付为 `incomplete`，human 反馈显示原因。

普通回归位于 `crates/codeguard-adapters/src/python_requirements.rs`、`crates/codeguard-cli/tests/detect_cli.rs` 和 `crates/codeguard-cli/tests/check_all_partial_contract.rs`。首次新增场景先因缺少 checker 发现和类别反馈而失败；标准 pylock 反例也先因未识别标准锁而失败，实施后目标回归通过。首次全工作区离线测试 161 组、968 通过、0 失败、101 条件忽略；标准锁改动后的完整终态另记于插件 OpenSpec 的 `verification.md`。

## 验收边界

`python.pip_audit` 目前只是候选检查器身份，项目实际配置、工具可用性、锁文件模型与完整依赖图均未证实。pylock 内容未按 PEP 751 解析，也未验证选用的环境、extras 或依赖组。尚无原生漏洞查询、有效 advisory 解析、数据库身份及时效、批准策略、稳定 CVE 任务或正式门禁。因此本验收不能证明“无漏洞”或 5.10/7.2 完成。
