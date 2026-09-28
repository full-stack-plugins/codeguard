# 全项目默认npm准备任务

支持范围：当前Unix CLI。check all默认把发现的npm根纳入共享任务图，缺上下文时调用准备服务，已初始化且输入可绑定则同步稳定任务并返回next；不安装工具、继承凭据或执行缺参数的原生命令。Java-only不调度npm；未初始化仅反馈workspace_not_initialized，不偷偷创建工作区。

RED：check_all_npm双根fixture删去显式Node参数后没有任务。修复：同一请求选择All且平台受支持即纳入npm根；原工具执行仍由明确完整参数控制。验证双根初次准备、重复准备、后续原生执行不堆积任务，并用trace证明准备阶段未启动工具。新增未初始化与Java-only反例。

47项不同普通回归通过，覆盖Java隔离、默认调度、预算/取消、坏清单/缺锁及SARIF。独立Draft202012验证实际默认多根反馈：首次两任务，重复零新增，交付incomplete。真实npm11.16.0/Node v24.18.0 check all回归通过（23.03秒），不安装或执行包脚本。其余7项忽略原生用例本轮未执行。Clippy -D warnings、fmt通过。

协议结构仍check_feedback0.25，不新增授权字段；准备不是原生执行证明。尚缺Windows路径、不可读/缺清单持久状态、可信上下文自动解析、所有语言/类别覆盖、宿主Hook、正式关闭及门禁。本验收不代表OpenSpec全计划完成。

后续进展：显式CVE/任务复检已支持npm-input-state-workbench.md所述不可用清单/锁状态；当前消费者check0.26/verify0.6/abort0.7。完整历史根恢复发现、外部工具/配置状态及宿主闭环仍缺。

后续有限恢复：npm-historical-roots-workbench.md已验收当前0.3初始化画像中清单不可用、物理目录仍存在的准备范围恢复；整体目录消失及画像刷新删除旧根后的完整义务恢复仍缺。
