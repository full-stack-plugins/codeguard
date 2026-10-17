# Clippy repair-ready 反馈局部验收

日期2026-10-06。对应现有OpenSpec9.3/9.9、11.17与hook-protocol；父任务未完成。

两个公开测试先RED：Rust编辑只给“Clippy待检查”而没有可执行后续指令；Clippy repair_ready摘要丢弃同任务规则和行号。现保留编辑阶段不编译的契约，明确项目Clippy未运行，并在批次后给出 `codeguard lint rust . --format=json`。原任务Cargo复检摘要增加规则和安全行号；父请求在子进程前后绑定静态输入集合及配置/锁，当前源码与工具摘要在投影前再次检查。RUSTUP_TOOLCHAIN与既有登记环境一起传递，不安装或切换工具链。

受控Cargo报告的发现→任务复检→源码修复→局部零诊断→复发保持同一任务。检查期间追加Cargo.lock内容的反例撤回所有位置、状态stale、观察incomplete。零诊断仍open，记录及任务清空不授予交付通过。

新Hook0.26/摘要0.8封闭schema保存历史协议，规则仅允许clippy命名空间，位置只有已验证行号，列单位unavailable。三份实际受控报告见[evidence](evidence/clippy-hook-controlled-2026-10-06.json)，不是语法oracle。协议正例及状态/列/规则/许可伪造反例两组通过。

WASM五个受影响目标51通过/0失败/6条件忽略，全workspace全目标严格Clippy通过。本机真实Cargo/Clippy单任务诊断已显式1通过；完成修复及复发后的末次实测独立记录。完整源码动态归属、features/target矩阵、批准来源、可信关闭、后台队列、真实宿主、多平台与发行仍缺，不勾选完整父任务。

末次本机真实Cargo/Clippy链路1通过（3.28秒），实际规则反馈与修复后反馈两份报告见[原生记录](evidence/clippy-hook-native-2026-10-06.json)。默认四个受影响目标47通过/0失败/6条件忽略，default/WASM严格Clippy通过；安装包扩展Rustfmt与Clippy链路1通过（31.99秒），源码/旧协议/用户Erlang草稿未被包测试改写。分层、OpenSpec strict、diff和实际报告schema验证通过；上一提交d6850bd的远端MSRV已成功，当前提交的CI另行核验，不把前一个提交当作本轮远端通过。
