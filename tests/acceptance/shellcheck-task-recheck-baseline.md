# ShellCheck 原工具任务复检验收

日期：2026-10-06。规格事实源：introduce-rust-codeguard-cli/native-tool-adapters，关联7.4与S09。范围为已初始化工作区的单文件原SC规则组复检；不是完整Shell能力或可信关闭验收。

## 行为和证据

`codeguard task verify CG-… PROJECT --shellcheck-tool /absolute/shellcheck --format json` 从首次私有报告及摘要绑定文件、方言、原显式rc和规则组。工具入口每轮重新核验。公开反馈为task_verification_preview 0.24.0，私有封套为shellcheck_task_recheck 0.1.0；与已有0.6预检失败反馈和其它语言协议分开。

真实 `/opt/homebrew/bin/shellcheck` 0.11.0 运行记录位于 `evidence/shell-task-2026-10-06-{first,present,rc-disabled,comment-disabled,fixed,fact}.json`。执行的最小源文件为bash shebang及 `echo $1`，修复改成 `echo "$1"`。临时工作区执行后清理，记录不包含源码和原生消息。

| 本轮条件 | 反馈 |
|---|---|
| 原SC2086仍存在 | still_present |
| 原rc新增disable=SC2086，零诊断 | rule_coverage_requires_review |
| rc恢复、添加disable注释，零诊断 | suppression_requires_review |
| 移除注释并引用参数，零诊断 | candidate_absent_unverified_policy |

四次复检均记录event_persisted=true；最终finding.state仍为open。注释检查是保守文本观察，可能命中heredoc中的注释外观，不证明实际有效抑制；只能要求审查，不能生成源码违规或关闭。

## 回归与边界

既有租约、失败尝试和next历史读取现能识别此封套及shellcheck时间序列。输入或配置改变后的历史不作为当前结果使用。新的复检不得伪造首次报告没有的SC规则。首次检查和复检复用zsh/fish文件名判断。不相关原生工具参数在租约前拒绝。

新增--ruff-tool反例先失败：参数被忽略并写入复检事件（`/private/tmp/codeguard-shell-wrong-tool-red.log`）。修复后相关目标通过。首次任务复检新增接线先因未知参数失败；第二次复检曾暴露历史消费者未识别新报告类型，该问题已修复。

默认全workspace：258个测试目标，1433通过、0失败、126忽略（`/private/tmp/codeguard-shell-task-workspace.log`）。WASM相关六个目标：71通过、0失败、13忽略（`/private/tmp/codeguard-shell-task-wasm.log`）；不运行用户正在维护的Erlang原生差分草稿。default/WASM workspace all-targets strict Clippy均通过（对应clippy-default/wasm日志），layering、OpenSpec strict、diff检查通过。修改的模块格式检查通过；lib.rs保持现有声明顺序，使用skip_children=true/reorder_modules=false/reorder_imports=false，不递归格式化他人文件。`tests/shell_lint_feedback_schema.py`四项通过，实际报告及假allow/假关闭/未知字段/空finding规则反例通过协议核验。

## 尚未完成

当前next 0.16指引仍提供原方言/rc的lint复扫，用户可显式执行task verify；自动选择专用复检的指引仍需完善。可信政策/覆盖的关闭、复发、项目全范围调度、zsh/fish专用工具、Dockerfile/IaC、跨平台、完整宿主和发行仍开放。父任务7.4与S09不勾选。
