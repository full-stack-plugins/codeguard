# 静态输入 FIFO 非阻塞拒绝

对应 execution-kernel 静态特殊文件输入约束及 S05/5.3 安装准备边界。

共享 read_bounded_regular_file 的 Unix 打开增加 O_NONBLOCK/O_CLOEXEC，保留 O_NOFOLLOW。在同一打开句柄上检查普通文件和大小，再作有界读取；无写入端 FIFO 不再先阻塞于 open。链接/超限/缺失的原有错误语义保留，配置或清单错误不转换成源码违规。

回归先在独立测试子进程复现 FIFO 阻塞：父进程两秒观察到未退出后终止并回收，测试失败，不留下后台进程。修复后同一用例通过，子进程实际运行读取函数并返回 InvalidData。补充 CLI 用例直接为 install 的锁及发行清单提供 FIFO；二者应在观察预算内结束，退出 3、状态 unreadable、writes_performed=false、readiness=unknown，不创建 codeguard 目录。CLI fixture 用系统 mkfifo 仅创建测试输入；产品实现仍是 Rust 文件 API。

最终 runtime 库 4 项、FIFO 1 项、CLI 23 项与 crate 边界 4 项，共 32 项通过，无 ignored。all-target Clippy（-D warnings）、格式、OpenSpec strict/diff 检查通过。

这只修复并验收 Unix FIFO 特殊文件拒绝，不保证任意挂起文件系统或阻塞设备的硬超时，不证明 Windows reparse point 处理，也不推进发行授权/实际下载/安装完成。完整安装及跨平台任务保持未完成。
