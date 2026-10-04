# P3C 部分执行的诊断与阻塞保留

日期：2026-10-04。规格事实源为 `introduce-rust-codeguard-cli` 的 verdict-integrity 双维结果及 remediation-workflow / RW21；关联 2.3、6.2、9.3、9.4、9.7、9.13、12.7。父任务仍未完成。

## 已复现的缺陷

受控 Maven 脚本输出新鲜、有效、范围与规则匹配的 PMD 6.15.0 XML 后退出 1。原探针保留诊断并准确返回 `incomplete / native_execution_failed`，但项目投影只接受正常完成的报告，导致稳定源码 finding 数为零，仅同步执行阻塞。计数没有冲突，不把任务遗漏描述成报告计数拒绝。

首个反例在修改产品代码前退出 101：`validated_diagnostic_survives_failed_native_execution_as_a_stable_task` 的平铺诊断数量实际 0、预期 1。日志 `/tmp/codeguard-p3c-partial-positive-red.log`。人类反馈缺少明确的原生状态/原因行也有独立 RED：`/tmp/codeguard-p3c-partial-human-red.log`。

## 最终行为与范围

- 有效且非空的正向诊断可以进入稳定任务；执行失败保留独立阻塞，失败文件不增加 `observed_file_count`。
- 重复部分扫描不重复建任务；正常扫描与失败扫描的同一源码/原生规则锚点复用任务。
- 原工具再次匹配问题可记录 `still_present`；任务简报消费同一复检结果。失败零诊断为 `incomplete`，执行阻塞仍为 `still_blocked`，已有任务保持 open。
- 坏 XML、外部文件、源码/配置/工具变化不进入当前源码任务。导入拒绝伪造原因、投影身份、完整计数或非所选规则。
- 两文件一正常一失败时保留两份有效 finding、一个执行阻塞，完整计数为 1/2。
- 既有 0.2 报告没有平铺失败诊断时，仍可按原来的阻塞语义读取。没有改写 schema、升级批准身份或关闭条件。
- human 输出明确原生局部状态、原因及保留的规则诊断；执行未完成仍返回 3。

## 实际反馈节选

来自 `/tmp/codeguard-p3c-partial-positive-observations/55660-80.json` 的字段节选；这是受控脚本的真实输出摘要，不是完整 schema 实例：

```json
{
  "command_status": "incomplete",
  "exit_code": 3,
  "local_status": "incomplete",
  "native_reason": "native_execution_failed",
  "finding_count": 1,
  "observed_file_count": 0,
  "local_observation_complete": false,
  "backlog_sync": {
    "failed_reports": 0,
    "historical_findings": 0,
    "imported_reports": 1,
    "new_blockers": 1,
    "new_findings": 1
  },
  "next_kind": "blocker",
  "delivery_decision": "not_evaluated"
}
```

下一步优先恢复检查环境，但已经观察的问题任务不会丢失。

## 验证

测试位于 `crates/codeguard-cli/tests/java_p3c_workbench.rs`，此批新增八个测试。六个相关默认目标实际 83 passed、0 failed、20 ignored，退出 0；其中工作台目标 21 passed、0 failed、0 ignored。日志 `/tmp/codeguard-p3c-partial-positive-related.log`。各分组重叠的再次执行不叠加为独立通过数。

串行默认全工作区退出 0：213 组、1193 passed、0 failed、109 ignored。随后七个相关 WASM 特性目标退出 0：89 passed、0 failed、20 ignored。28 份实际单文件反馈通过既有 schema，12 个伪造通过变体被拒绝；198 个 schema 元定义有效且原件没有修改。fmt、分层门禁、OpenSpec strict 及文档局部路径检查（687 条）通过。全工作区全目标 WASM Clippy `-D warnings` 退出 0。

复现命令：

```bash
cargo test --workspace --all-targets --locked --offline
cargo test --locked --offline -p codeguard-cli --features wasm-precheck \
  --test java_p3c_workbench --test java_p3c_cli --test check_all_java_p3c \
  --test task_verify_contract --test work_sync_contract --test next_command_contract \
  --test java_syntax_fallback_candidate
cargo clippy --workspace --all-targets --locked --offline --features wasm-precheck -- -D warnings
cargo fmt --all -- --check
python3 scripts/check_layering.py
openspec validate introduce-rust-codeguard-cli --strict
```

默认和 WASM 构建必须串行，避免共享 `target/debug/codeguard` 被另一模式覆盖。Python 只用于现有开发 schema/分层验证，不进入产品运行时。

| 证据日志 | SHA-256 |
| --- | --- |
| `/tmp/codeguard-p3c-partial-positive-red.log` | `9b76d634e0d6e9cac00a761affeb792d3e30024c1b8da3a6d232845aa5e9ace4` |
| `/tmp/codeguard-p3c-partial-human-red.log` | `2b956eee03cc91892493a802c2cd66c762eec03811665b77860f9b97496411df` |
| `/tmp/codeguard-p3c-partial-positive-workspace.log` | `3c68b970bc5f662ece9d3d2d099193db98d3114aa832199e33bcefa250e84760` |
| `/tmp/codeguard-p3c-partial-positive-feature.log` | `1d3ff39dd99645c62f4b2b75f98d355b937e90f528175a93a046d156d5e436a1` |
| `/tmp/codeguard-p3c-partial-positive-schema.log` | `bc0f153291108d03207e9a39ea723a494f4a9c9b6c9f83a11c703933c1330472` |

Clippy 日志 `/tmp/codeguard-p3c-partial-positive-clippy.log` SHA-256 `5899138ea698567f7ff79bdf7a23d413b2460d5f973b290a5eb1c8d3ac98fc49`。

此批 Maven/JDK/仓库是协议替身，不是真实 P3C 引擎；它验证结果投影、任务、原工具复检入口和导入边界，不证明 56 条 P3C 规则精度、项目生效模型、可信关闭/复发、全语言门禁、宿主或公开 npm 发布。既有 Erlang 37 例 RED 草稿保持未提交，不计本批通过。
