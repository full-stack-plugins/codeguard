# ESLint 局部任务复检验收

公开task verify支持node.eslint和node.eslint.preparation。需显式--node-tool、--eslint-entry、--eslint-version、--config、--cwd；未知/重复/非法选项在执行前拒绝，不从Markdown或历史报告选择程序。任务复检沿用原租约、统一截止时间、同步和失败尝试预算。无上下文返回明确未完成并保存开放观察，不运行其它语言工具。

Rust复用同一ESLint probe，在临时原生输出退出清理前捕获摘要绑定的局部报告。复检封装绑定任务/范围/规则/工作区、本轮报告与首次工具/原配置/cwd连续性，严格同步后保存verification_observed事件。next、status和尝试历史均能读取ESLint时间顺序和报告类型；新复检与新问题引用的嵌套报告可继续构造原工具指引，不能退回旧源码方向。这里只持久保存脱敏观察，不声称保留完整原生证据或可信批准。

分类为仍存在、局部消失候选、未完成、抑制需复核、原上下文变化需核查，准备任务局部恢复仍未批准。任何结果不写resolved或交付allow。缺上下文入口先RED（旧task_checker_unsupported）；新事件接线又暴露尝试历史不认识ESLint报告/时间的问题，修正后同任务查询与重复扫描通过。

57项相关普通回归通过（ESLint6、next6、status/show4、租约/尝试16、复检10、同步15），另1项分类单元用例覆盖上下文变更、原问题仍存在、抑制与目标错误。CLI协议夹具验证显式原上下文复检still_present及未完成事件保存。实际现有Node24.18.0/ESLint10.11.0运行公开发现、task verify仍存在、同配置修复后复扫、task verify局部消失四轮，194.36秒通过，两种任务复检事件均保存、任务不自动关闭、交付未评估，无安装/升级。

公开未完成及原生观察两类task verify输出均通过task-verification-preview和eslint-task-recheck的Draft202012 schema校验。原统一schema未识别ESLint报告的缺口由实测发现并补入严格类型分支。最终ESLint/verify16项回归、目标Clippy -D warnings及fmt通过；OpenSpec严格校验及插件diff检查通过。

受批准required rule集合、JS导入闭包及完整配置覆盖、TS插件/项目源集、正式关闭重开、可信宿主策略与全量门禁仍缺；candidate_absent仅供调查。7.3/9.7继续未完成，不把本验收当整个Node质量守卫完成。

后续复检协议升级0.2.0，新增原规则有效配置查询；上述0.1验收记录仅为当时结果。当前新增原生反例、配置关闭分流与schema复核见eslint-effective-settings.md，仍不授予正式关闭或白名单批准。
