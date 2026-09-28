# ESLint 输入绑定执行验收

Rust run_eslint_probe 接收显式 Node/ESLint JS 入口、原 flat config、源集、私有证据目录、本轮报告槽位、精确版本与完整输入摘要。统一 runtime 执行版本探测和扫描，共享绝对截止时间、取消与有界日志。所有显式路径须为物理绝对路径；输入摘要集合必须精确覆盖 Node、入口、配置及源码。

报告启动前不得存在，文件名与安全 run_id 绑定。原生版本输出须严格匹配冻结版本；版本探测后和扫描后重新校验输入。解析后的 warning/error 只是局部观察，local_coherent 不等于质量通过；坏报告、缺报告、错版本、输入变化或取消/超时均不能标检查成功或源码修复。

## 用例

缺 API 的 RED 编译失败后接通公开执行入口。普通契约覆盖缺配置摘要、错工具摘要、非支持版本、旧报告、不安全 run_id、预先取消和截止时间耗尽，均在 Node 启动前拒绝。

显式实际 Node 执行项目自有受控 JS 夹具，覆盖干净报告、warning/退出0、error/退出1、坏 JSON、版本失配、扫描时改配置及退出2/缺报告。该脚本不含 ESLint；这是实际进程与 JSON 协议测试，不是 ESLint 原生验收。正常夹具预算设为60秒，避免调试构建的重复 Node 制品摘要计算耗尽原10秒预算；截止时间拒绝另有普通反例。

## 未完成边界

未安装工具。ESLint 原生执行 NOT_RUN。此请求不证明 Node 包、JS 导入及插件闭包、TS parser/tsconfig、项目生效配置、完整源集或工具批准。公开 CLI/持久任务/正式门禁尚未接线，7.3 保持未完成。不能据局部一致结果生成已批准白名单或关闭任务。

最终19项普通目标/回归通过，实际 Node 七模式测试113.72秒通过。CLI lib/新执行测试和 adapter 全目标 Clippy、fmt通过。OpenSpec严格校验与插件差异空白校验通过。

## 运行中取消与超时补充

新增 Unix 受控可执行夹具四种模式：扫描已经实际启动后取消/超时，各自分别已生成有效零诊断报告和尚无报告。用扫描标记及报告实际存在性确认前提。该夹具不冒充 Node/ESLint 原生兼容验收。

报告缺失加取消先出现 RED：执行器返回一般 eslint_report_or_execution_incomplete，丢失 request_cancelled。实现现在优先采用实际请求中断及 runtime 终止原因；其它失败按 preflight、process evidence、report read 分流。四种模式均保持未完成，不建议修改无关源码。版本留证失败也保留取消/超时原因。正常退出2且缺报告明确返回 eslint_report_read_failed，不当作源码规则错误。

中断修正后的18项普通相关回归通过；四模式中断测试3.28秒。实际Node七模式JSON夹具复跑109.10秒通过；CLI相关Clippy、fmt、OpenSpec strict和插件diff检查通过。完整原生ESLint/项目配置/门禁验收仍未完成。

后续原生验收已使用本机既有ESLint10.11.0执行六轮核心规则/解析/抑制/修复复检，见eslint-native-core-rules.md。前文NOT_RUN为当时状态；仍不证明完整项目上下文、TS插件或正式门禁。
