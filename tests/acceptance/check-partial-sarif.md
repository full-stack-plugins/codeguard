# `check all/java --format sarif` 局部反馈验收

当前 `check all/java` 仍生成局部 `check_feedback`/`check_aborted`，尚未生成完整 RunReport。SARIF 输出直接由同轮局部报告投影，不能把它解释成完整项目质量认证。

已验证：

- 真实命令 `check all --format sarif` 保留原生 Clippy finding，退出 3，SARIF `executionSuccessful=false`、交付为 `incomplete`；原生消息中的模拟凭据不进入公开输出。
- `check java --format=sarif` 在零 finding 时仍退出 3、标记执行未完成，交付为 `not_evaluated`。
- 内部故障报告仍把兄弟检查已取得的 finding 留在 SARIF；输出只含散列规则/发现身份，不复制原生消息、源码路径或证据文本。
- `--output PATH` 对 JSON/SARIF 使用同目录暂存写入并原子替换；成功导出的文件与 stdout 是同一文档。目标父目录不可用时仍把已取得的 finding 留在 stdout，退出 3 并在 stderr 给出恢复原因。已有源码文件不能被误当作报告覆盖；已有可识别 CodeGuard 报告可以替换。
- 导出状态也写入结构化反馈：局部 JSON 0.18 与故障报告 0.3 的 `export.status` 为 `not_requested/saved/failed`，失败时附受限原因码；局部 SARIF 的 `codeguardExportStatus/Reason` 给出同义投影。成功文件与 stdout 都标记 `saved`；写入失败只有 stdout 标记 `failed`，不会伪造已保存文件。

定向命令：`cargo test -p codeguard-cli --test check_sarif_cli --offline`；`cargo test -p codeguard-cli --lib partial_sarif_feedback --offline`。

局限：当前仅投影局部报告中的原生 `findings`；依赖图节点和未归属的 CVE advisory 不是已确认 finding，不自动转成 SARIF 结果。报告缺可信质量策略和完整义务账本，永远显示未完成；`--output` 暂只接入 `check all/java` 的 JSON/SARIF 局部报告，human 与其它检查类命令尚未接入。正式 RunReport、白名单批准、MCP/Hook、可逆非 UTF-8 路径及完整门禁仍待后续任务。
