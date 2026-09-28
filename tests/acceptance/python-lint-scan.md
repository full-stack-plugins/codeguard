# Python Ruff 项目扫描与对话反馈切片（2026-09-24）

本切片将 `detect` 已观察到的 Python 源文件逐个绑定到项目 Ruff 配置，再调用原生 Ruff；没有项目配置的文件保留 `missing`，工具身份错误、原生排除文件等保留 `incomplete`，有效问题与未完成原因可同时返回。默认跳过 `.venv` 等点目录源码，但仍发现 `.ruff.toml`。扫描结果通过 `python_lint_feedback` 转为智能体可显示的 JSON：配置状态、每文件运行状态、规则 ID、行列、修复建议及原工具复检 argv；原始诊断文案不直接进入对话，交付决定固定 `not_evaluated`。

`cargo test --offline -q -p codeguard-cli --test python_lint_scan_contract`：3 项通过，3 项外部工具测试忽略。`CODEGUARD_RUFF_BIN=/opt/anaconda3/bin/ruff cargo test --offline -q -p codeguard-cli --test python_lint_scan_contract -- --ignored`：真实 Ruff 0.16.8 的 3 项通过，覆盖项目 E501 与干净文件共存、排除文件导致未完成但保留其它有效诊断、根/嵌套目录按各自生效配置得到 E501/F401。`detect_cli` 的点目录测试也通过。

原生 JSON 的 `filename` 现在必须解析到本次请求的源文件；混合报告中属于其它文件的诊断不能被误归因，同文件诊断仍保留且整次状态为 `report_source_mismatch`/未完成。受控假原生报告先使 `ruff_probe_contract` 反例失败（旧实现误判 `EvidenceComplete`），修复后反例与真实 Ruff 回归通过。路径归属只用于本次局部范围，不宣称已解决并发同用户篡改或完整项目快照。

`codeguard lint python [path] [--ruff-tool ABS_PATH] [--format human|json]` 现把这条链路作为公开的局部命令输出。它先发现项目配置；只有存在已配置的 Ruff lint 时才查找并调用原生工具。未安装、无法读取、版本异常或原生执行失败均进入反馈的未完成原因。输出带本次 `run_id`、配置与运行状态、规则摘要和复检信息，`tool_approval=unverified`、`delivery_decision=not_evaluated`，退出码固定为 3，避免将局部扫描当成完整质量通过。临时原始日志在本次命令结束后清理。

`cargo test --offline -q -p codeguard-cli --test lint_python_cli`：5 项通过、2 项真实工具测试忽略；`CODEGUARD_RUFF_BIN=/opt/anaconda3/bin/ruff cargo test --offline -q -p codeguard-cli --test lint_python_cli -- --ignored`：真实 Ruff 2 项通过，显式工具路径及从绝对 PATH 目录发现工具均把 F401 返回 CLI JSON。缺配置、缺工具、错误程序伪装、human 输出及不支持语言均有独立用例。

尚无完整质量策略、工具批准来源、其它 Python 检测类别或 Codex/ZCode/Kimi 宿主 Hook 接线。CLI 结果已能由调用它的智能体读取，但这不等于三个宿主自动将结果显示在对话界面；OpenSpec 5.10、7.2、11.16 保持未完成。

## 2026-09-25 局部修复提示与扫描后输入变化

`python_lint_feedback` 0.2 对原生完成的 F401/E501 给出限定文件、扫描前源码 SHA-256、谨慎修复步骤和原生复检条件；未知规则只给调查步骤，原始诊断消息仍不进入对话。提示不是持久 RepairBrief、批准规则或任务关闭依据。局部反馈仍固定 `delivery_decision=not_evaluated`。

逐文件扫描结束后再次核对源码和 Ruff 配置。若扫描后被其他工具改动，受影响文件转为 `incomplete`，已有诊断仅作待复查证据，提示变成 `verification_required`；不能据旧输入指导直接修改。模拟第二个文件检查时修改第一个文件的原生工具回归通过。`python_lint_scan_contract` 5 项普通测试与固定 Ruff 0.16.8 的 3 项显式原生测试均通过；完整工作区验证另记于 OpenSpec verification。

## 2026-09-25 Ruff 局部稳定发现身份

局部反馈升为 0.3，对原生完整且源码摘要未变化的 Ruff 诊断提供 `finding_id` 和完整 `finding_fingerprint`。身份由检查器协议、项目内路径、原生规则、去首尾空白的源码行字节、原生诊断文字摘要及重复序号构成；行号只用于展示。相同问题前面插入其它行后 ID 保持稳定，规则或路径变化不会错误复用；同一行同一诊断重复出现也有不同 ID。原生文字只参与摘要，不回显或当作指令。无法读取定位行、扫描未完成或扫描后输入变化时不签发该身份。

固定 Ruff 0.16.8 的真实双轮扫描验证了行号移动但 ID 不变、文件字节摘要变化。两项纯身份单测与 `python_lint_scan_contract` 5 项普通/3 项原生测试通过。此 ID 仅是未来 `work sync` 的候选键：尚无持久 finding、重命名关联、跨工具归并、冲突协调或复检关闭，不能据此声称 OpenSpec 9.3 已完成。
