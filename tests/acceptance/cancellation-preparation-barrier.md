# 取消测试准备屏障纠偏

受控 Cargo 夹具延迟 2.2 秒，先复现旧两秒齐备断言失败。失败报告的 started_native_task_count=2 表明两个任务确已启动；Rust 标记写在诊断输出后，不能作为启动时间的证据。该复现确认夹具混淆启动与诊断准备，不证明此前偶发失败的主机负载根因。

修正只涉及测试夹具：准备等待窗口与执行预算分开，Python 迟到写入等待独立屏障；Rust 诊断准备后才解除屏障并发送取消信号。缺少标记时中断并回收子进程，附带实际输出诊断。继续要求退出 130、保留 Rust finding 和发现范围、报告两个任务已启动，并确认 Python 子孙进程不能产生迟到写入。原生产取消实现未修改，专用截止测试仍保留。

默认并行执行 check_all_partial_contract：13 项通过、1 项真实 Ruff 未执行。随后与 check_all_java_p3c、java_javadoc_cli 一并回归：41 项通过、8 项真实工具测试未执行。CLI all-target Clippy -D warnings 通过。未运行全 workspace；不把一次通过作为所有平台与负载下无偶发失败的证明。
