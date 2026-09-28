# ESLint 公开局部命令反馈验收

本文件记录最初0.1入口验收。后续0.2显式工作区入队、稳定任务与next连接见[工作台验收](eslint-workbench-sync.md)；下文未接入工作台是当时状态，不代表当前全部入口。

公开入口：codeguard lint typescript FILE --node-tool ABS_PATH --eslint-entry ABS_PATH --eslint-version VERSION --config ABS_PATH --cwd ABS_PATH --timeout 60s --format json。human/json可选，参数未知/重复、非法版本/路径/预算为用法错误2；缺原生上下文为准备反馈3，不制造源码finding或默认规则。

必须明确原工作目录，不能从配置位置猜测原生 --config 的相对匹配上下文。当前仅显式单文件；目录范围返回准备原因，不宣称项目已扫描。版本、原配置、源码/工具摘要、新鲜报告与取消/截止时间沿用统一Rust probe。局部诊断保留规则、位置和严重度，原生敏感消息只输出摘要；反馈协议0.1.0及schema固定coverage_proven=false、delivery_decision=not_evaluated、workbench_status=not_connected。临时日志/报告退出后清理，不能声称原生证据已持久同步或关闭了任务。

缺上下文用例先RED（旧路由返回用法错误2），接通后返回明确准备状态。三项普通CLI契约覆盖缺上下文、非法/重复参数和受控原生JSON到反馈（原生消息不泄露、不创建codeguard状态）。实际现有Node24.18.0/ESLint10.11.0执行公开命令两轮：no-debugger原生finding，改源码后原配置零诊断；35.44秒通过，两个结果均退出3且门禁未判定。未安装/升级工具。

最终23项普通相关回归通过（新CLI3、probe2、Go5、Python12、语言缺口1）。相关CLI lib/bin/测试Clippy、fmt通过；公开缺上下文输出以Draft202012 schema校验通过。OpenSpec strict和插件diff检查通过。

原生工作台持久化、完整项目/批量调度、JS配置/导入闭包与可信工具身份、TS parser/tsconfig/插件、monorepo及依赖安全仍缺，OpenSpec7.3保持未完成。这是公开局部入口验收，不能当成全部Node质量守卫交付。
