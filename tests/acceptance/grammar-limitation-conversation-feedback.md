# 具体 grammar 限制的终端与对话反馈

日期：2026-10-05。对应既有 OpenSpec 14.7、14.8、14.19 的局部验收；不以反馈修复完成父任务。

## 真实缺口与 RED

固定 Python grammar 清单的第二条已记录 Python 3.14 模板字符串兼容问题，但终端只显示第一条通用 smoke 说明；Claude 摘要不显示具体限制。两个新增实际入口回归在修改前分别失败，退出 101：human 缺 `Python 3.14 template strings`，Claude 也缺该提示。输入为含私有内容的模板字符串，测试要求任何公开反馈都不含该私有内容。

## 行为

聚合终端每个候选最多显示三条固定限制，每条最多512字符，最多八个候选条目。Claude 只按本轮实际 observed 候选的语言从程序随带清单读取限制；不采用报告提供的任意文本或源码/路径。每个语言显示最后追加的限制片段，最多两个不同语言，每条180字符，重复语言归并。此读取复用清单元数据，不重新运行检查器或解析源码。

摘要仍要求原生 lint/编译器确认，不把兼容限制当作白名单、语言版本推断或已修复。完整摘要不超过1200字符，预留末尾未验收/未评估说明，不能因截断丢失边界。任务、恢复节点、结构观察、输出协议和退出语义不变。

## Python 漏报与结构观察的边界

原始 grammar 对空函数体与错误缩进仍无 ERROR/MISSING，原生差分保留 FN2。既有 `codeguard.python.required_suite` 独立结构规则可将这类样例导向原生确认；它不是原 grammar 修复，不能把结构规则伪造为恢复节点。当前回归继续验证结构观察、稳定任务和 Hook 反馈，不把“零恢复”理解为源码正确。

## 反例

私有源码不进入终端或 Claude 摘要；伪造的报告限制文本、未知语言及不可用候选不提供提示。固定清单不是用户批准来源，也不赋予放行权威。两个元数据单元回归已通过；实际入口和受影响完整回归结果随本次验证补充。

## 当前验证结果

WASM 元数据单元2项通过；五个完整目标：check_all_grammar_candidates 14、check_python_structure 1、claude_hook_cli 12（原生显式目标1项暂忽略）、hook_syntax_tasks 17、python_structural_syntax 2，共46项通过。最后新增的摘要尾部断言独立重跑1项通过，摘要<=1200且未验收边界始终在末尾。

显式提供已安装 Ruff 0.16.8 后，忽略目标 `real_claude_write_event_injects_native_rule_without_raw_tool_message` 实际执行1项通过（6.13秒），验证原生反馈仍优先、原生消息不泄露。本机通过 manifest CLI 形式重放 Claude 事件，不等于实际 Claude 应用自动触发。OpenSpec strict、定向 fmt、crate layering 和 diff 检查通过，完整远端 CI 仍待当前提交终态。

默认与 WASM 全目标严格 Clippy 均通过。默认构建元数据单元回归也执行，固定清单与前序语料/原生报告原字节未变，受保护 Erlang 草稿摘要未变。
