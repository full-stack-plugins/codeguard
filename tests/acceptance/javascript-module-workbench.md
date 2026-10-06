# JavaScript 声明模块的项目检查与稳定任务接线

延续同一 OpenSpec change 的 syntax-precheck / Project fallback binds module evidence to stable native-confirmation tasks，以及14.x/9.x/11.x。项目 `check all`、独立 `lint typescript`、`lint all` 与编辑 Hook 共用原生优先的候选规划；仅显式 `.mjs` 或最近包明确type=module的整文件 `.js` 选择模块worker。CommonJS和未知模式不启用函数外return规则；worker之后同时复核源码与模式证据，变化返回未完成。原生ESLint已覆盖目标仍优先，不重复WASM。

先执行新增端到端测试，确认缺少module候选导致RED；接通后验证两个module路径均发现函数外return、`.cjs`不报该候选、项目重复扫描/独立lint/lint all/编辑复用同一任务。清洁源码保留原任务open；五类首导入伪造（包摘要、模式、搜索目录、规则摘要、旧0.13协议无模式携带模块规则）全部拒绝，不生成源码finding。包类型改为CommonJS后，原任务复检返回javascript_module_context_changed，受控假Node没有被启动。此为故障边界测试，不冒充真实Node/ESLint运行。

协议：check_feedback0.60同时保留普通check与lint-only封闭分支；后者仍必须requested_categories=[lint]，简报只能来自lint允许集合。持久syntax确认0.15、ESLint反馈0.7、Hook外层0.29/局部0.17、module简报0.21保留独立契约，旧版本不改。额外发现旧聚合schema不消费已有npm准备简报，本批用专门封闭投影补入0.60普通check分支，不放宽lint-only。实际七份报告通过schema，六类模式/身份/规则/原生执行伪造被拒绝；408份schema元定义通过。

五个相关目标18 passed/0 failed/5条件忽略（实际原生条件未在本批执行）。[数据摘要](evidence/javascript-module-workbench-2026-10-06.json)、[实际报告包](evidence/javascript-module-workbench-reports-2026-10-06.json)绑定本批输出。Hook回归和最终静态检查结果随后追加；历史完整回归不作为本批全仓验收证据。

本批只完成声明module的局部链路，不勾选完整14.x或9.x/11.x，不改变固定358例指标，不批准独立holdout/32语言资格、可信全量关闭或宿主发行。当前资格仍0/32，新增15.1–15.7四核心能力生产门槛全部未完成。

## 最终验证

五个Hook/项目候选/next回归目标71 passed/0 failed/3条件忽略；与上述18项为不同目标，本批合计89 passed/0 failed/8条件忽略。默认与WASM两种CLI全目标Clippy -D warnings通过；分层、OpenSpec strict、diff检查通过。未执行受保护Erlang草稿，摘要仍2e3a296072e6a3aa34b561799c18c7d8b621d00f8786301d2612cee8cdab85b6。该本地证据不代替远端CI、全工作区、原生工具矩阵或生产验收。
