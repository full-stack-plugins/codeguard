# Rustdoc 自动持久同步与下一步验收

2026-09-27；沿用OpenSpec introduce-rust-codeguard-cli。7.1/9.4/9.7仍未完成。

报告0.4绑定当前初始化工作区、独立run_id及原生目标相对路径。自动保存queued报告并调用work sync，导入重建源码范围候选身份、简报及当前源/清单/锁身份。完整且唯一的当前发现保存稳定fact、Markdown任务和观察；重复检查不重复建任务。未完成只创建环境/身份准备任务；陈旧报告不生成新的源码任务，保留历史并要求重扫。篡改范围/指纹、错误状态及额外字段拒绝导入。next识别文档任务并要求原工具重新检查当前输入，不把首次源码相同视为配置仍有效。

TDD：初始化契约先失败于not_integrated，接通后通过。四项新契约覆盖重复检查与next、缺工具的非源码准备任务、排队后清单变化、伪造指纹被拒。相关普通回归36项通过：CLI11、身份3、同步15、跨类别1、next6；普通执行忽略三项原生测试，不算通过。真实Cargo CLI独立1项通过，5.56秒，含初始化、缺文档真实任务、修复后零诊断及中文坏链接；零诊断不关闭旧任务。相关Clippy -D warnings退出0，10.28秒。真实绑定反馈及queued报告通过Draft202012，任务文件存在，缺工作区ID反例被拒。Python仅用于独立协议验收，产品运行全Rust。

复跑：cargo test -p codeguard-cli --test rust_comments_cli --test rustdoc_identity_contract --test work_sync_contract --test work_sync_cross_category --test next_command_contract。原生Rustdoc需显式CODEGUARD_CARGO_BIN再cargo test -p codeguard-cli --test rust_comments_cli -- --ignored；无需安装。

尚缺文档task verify、持久尝试、抑制对照、正式关闭/复发、完整符号归属、原配置/全部工作区/features/targets、可信工具/规则与宿主。没有全工作区回归或发布验收。本次自动同步是部分闭环，不能宣布整个计划完成。
