# Zig 聚合与编辑原生路由验收

对应 `introduce-rust-codeguard-cli` 9.9/9.10、14.7/14.9/14.14/14.19 既有范围；不将单文件AST证明扩大为完整语言验收。

两个聚合反例先失败：显式Zig参数未接入，PATH工具可见仍无原生报告。修正统一check和Hook调度后，限定源码快照、冻结入口目标、共享截止时间、最多64文件；失效定位撤回、选定失败不改用WASM，缺工具仍有候选观察。

check反馈0.46 / Hook执行反馈0.14 / 快检0.6 / Zig scan0.1独立协议；历史协议原件不改，无Zig仍沿用旧报告。人类终端和Claude摘要保留原生规则与位置。Zig原生首次任务尚未接入共享导入器，逐文件task_id=null，next仍不签发关闭；原生首次持久任务与完整lint/build继续未完成。

实际已安装Zig0.16.0、限定check zig：非法样本diagnostics_observed，修复样本completed，退出均3、交付未评估，WASM没有代替原生。显式真实测试1 passed/0 failed/0 ignored，最终7.13秒只是本组观察。报告见[evidence](evidence/zig-aggregate-2026-10-05.json)。

回归与协议校验终态在完成后追加。公开npm和插件锁未升级；CLI转换输出不能替代实际已安装宿主验收，32grammar资格仍为0。

最终验证：首轮受影响WASM八目标71 passed/0 failed/5条件忽略；参数对象与SARIF投影修改后，最终WASM四目标41 passed/0 failed/4条件忽略，默认Zig/SARIF四目标20 passed/0 failed/1真实工具条件忽略。覆盖重叠不重复累加。实际Zig另1显式通过；5协议/状态负例通过，包含拒绝有诊断的completed和过时位置。默认/WASM workspace/all-targets严格Clippy、fmt、分层、OpenSpec strict、244历史schema原字节和668文档本地文件链接通过。完整workspace默认suite未重跑，旧结果不作为当前全套证据。

SARIF会保留当前Zig原生诊断，使用私有证据引用约定而不公开源码路径或消息；executionSuccessful=false，不把局部AST成功写成项目通过。

聚合中止增量：其他任务内部失败时，当前Zig结果独立保存到0.15中止协议并追加对应执行状态；默认与WASM中止构造器各3测试通过。新协议兼容、旧消费者拒绝及禁止allow由单独协议负例覆盖。此处是构造器和协议验收，尚未用真实混合项目故障注入证明整个中止运行路径。
