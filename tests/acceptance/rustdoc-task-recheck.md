# Rustdoc 任务复检验收

2026-09-27；OpenSpec introduce-rust-codeguard-cli；7.1/9.7保持未完成。

复用公开comments rust服务的固定锁定离线原生库目标机器流。task verify在共享预算及任务租约下运行普通rustdoc与force-warn对照，捕获全部已发现Rust源/根清单/锁，核验两轮源集合、输入及工具摘要；复检事件和已准备的attempt绑定同一稳定任务。next/历史读取支持新的报告类型及时间序列。输入已变化时不沿用旧观察；任务保持open，不生成批准白名单或交付通过。

分类：still_present、rule_coverage_requires_review、suppression_requires_review、candidate_absent_unverified_policy；准备任务恢复也只environment_restored_unverified_policy。原生报告坏、输入变化或执行失败为未完成。原始观察封套rustdoc_task_recheck 0.1；文档任务预览0.7，保留旧0.6schema和具名旧协议。

RED：缺任务接线时task_checker_unsupported；接线后第二次复检因历史读取器未接入checker及序列格式失败，补齐后连续仍在/抑制/缺失通过。普通相关59项不同测试通过（CLI12/next6/work sync15/lease16/verify10），11项默认忽略不是通过。原生独立1项通过，14.07秒，包括初始化稳定任务、真实缺文档复检、补齐文档、源码allow和force-warn反例、中文坏链接；多轮不当作多个测试。实际原生任务预览及普通/强制嵌套观察通过Draft202012。校验Python只属于独立验收，产品全Rust。最终Clippy/fmt见插件verification.md。

复跑：cargo test -p codeguard-cli --test rust_comments_cli --test next_command_contract --test work_sync_contract --test task_verify_contract --test task_lease_contract。显式既有CODEGUARD_CARGO_BIN后cargo test -p codeguard-cli --test rust_comments_cli -- --ignored运行原生验收；未安装。

未完成：正式关闭/复发重开、完整符号归属/原配置/全部workspace/features/targets、可信工具规则批准、跨平台宿主及全语言类别；旧工具观察不证明未来工具身份或批准。全工作区未重跑，完整计划仍须继续。
