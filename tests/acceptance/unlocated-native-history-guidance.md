# 未定位任务的原生历史指引回归

日期：2026-10-04。对应既有 syntax-precheck、修复闭环及宿主反馈任务。首次原生扫描与白名单期限是独立增量；本记录不将单一缺陷修复等同完整验收。

GitHub CI 37210749865（提交520daf6）在 hook_syntax_tasks 的 unlocated_recoveries_become_stable_environment_tasks_without_source_positions 失败。Kotlin 的首次 WASM 恢复不可定位，重复保存生成缺工具原生历史；task show 只投影新的环境恢复文字，遗漏首次“无法定位、原生确认前不得修改源码”的约束。本机按同一原有断言复现失败，没有删除或放宽断言。另增加源码变更后的过期历史检查，也先失败。

现在 next/task show 从已绑定首次0.3报告保留未定位限制，并叠加当前具体环境恢复步骤。原生观察未完成或输入过期时禁止在确认前修改源码，不伪造位置；当前原生检查确认语法诊断后仍可按有界位置修复；当前原生零诊断继续正式策略与覆盖核验，不自动关闭。新文字没有改变旧 schema、任务身份、原生执行或批准权限。

实际命令反馈另保存于[缺工具与过期简报](evidence/unlocated-native-history-guidance-2026-10-04.json)。该记录执行真实隔离WASM与CLI，没有安装或运行原生 Kotlin 工具，也不是实际宿主市场安装验收。

后续须独立等待新提交的Linux CI终态；失败提交的CI不能改记为成功。完整语言质量、可信关闭、真实宿主与发行任务继续开放。

## 最终本地验证

- Hook任务15项、Kotlin首次扫描5项、Kotlin任务复检3项，均0失败/0忽略。保留原断言并增加两份Kotlin源码变更后的过期检查，没有修改Erlang保护草稿。
- 实际两次Hook、两份task show外层及内层简报均通过现有封闭协议；225份schema字节不变。实际WASM/CLI运行程序前后SHA一致，不将测试替身当作原生工具确认。
- 默认/WASM全目标CLI Clippy -D warnings、fmt、分层、OpenSpec strict通过。没有宣称全workspace本地复跑或当前新提交远端成功。

日志身份：

- `/tmp/codeguard-ci-37210749865-failed.log`：`0dedc9dd9481c8741b2a32b698b1431096abddef12f969cd6ff21ebcf0b5e199`。
- `/tmp/codeguard-unlocated-guidance-regression-red.log`：`5f966a5c1d60c000fe0cd2ed15b344784b9541b618cea157aa134a3a71fdf2fd`。
- `/tmp/codeguard-unlocated-stale-guidance-red.log`：`c4daca9a6d99e6eec081281747d93baf98892f3a2799e0b510a91f0a44a3d056`。
- `/tmp/codeguard-unlocated-guidance-final.log`：`d831fb5e1237e0bc844d03c24d0ccfeb08ae8167f3812b9a2cba4f12c93c06c2`。
- `/tmp/codeguard-unlocated-guidance-actual.log`：`032c22799bf120b471662c924ba1102d4dd0e2fadab5c2207098a39731d6fd88`。
- `/tmp/codeguard-unlocated-guidance-default-clippy.log`：`9b6b78de2b11b792e98045cd62d5dcb1e2890d4948bfee8d11286e6ff4eba42b`。
- `/tmp/codeguard-unlocated-guidance-wasm-clippy.log`：`2ab49cad37111ac1c26776c057c30ea510a9155d2c34e78606d6f08dfb7009a1`。
- `/tmp/codeguard-unlocated-guidance-openspec.log`：`cf3ec12601576e4a3dd335699d867edc30029dd07ee9fdcd6396379524b1fa4b`。
- `/tmp/codeguard-unlocated-guidance-layering.log`：`280c626d05d03e0944fca548fce5a1e9039200ab2cdd1f3d68f64c12e236de36`。

实际报告SHA-256：`75cec5456a126c008ded321a2e5b528c05727f616aa79e3f55ae5c41f88b0448`。
