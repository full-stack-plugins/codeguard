# comments rust 局部原生入口

规格：introduce-rust-codeguard-cli / Native evidence SHALL be interpreted per tool contract，任务7.1。

命令：`codeguard comments rust [path] [--cargo-tool ABS_PATH] [--timeout DURATION] [--format human|json]`，当前Unix入口。显式Cargo经共享runtime调用rustdoc --lib --locked --offline机器报告，探针规则为missing_docs和broken_intra_doc_links；这些规则不冒充项目获批准的文档策略。私有target目录在退出后清理，不用项目target输出。不存在可读锁文件时不生成新锁或进入原生执行。

共享预算按CLI/登记环境/项目默认/内置选择。SourceSnapshot绑定已发现Rust源码、根清单和锁文件；执行后复核文件字节和源码集合。Cargo入口字节前后核对。原生finding须来自本根清单、已观察源码及库目标，目标/内容变化为未完成；规则与主位置公开，原始诊断文本和package路径不投影为智能体指令。找不到工具、原生失败、坏机器报告、超时/取消或来源失配不能变成注释违规或通过。

报告rustdoc-local-observation:0.1.0固定authority=local_unverified、coverage_proven=false、delivery_decision=not_evaluated，普通操作退出3，取消130。局部扫描完成仅说明这次观察契约成立；源码allow仍可能隐藏规则，零诊断不能证明修复。backlog_status=not_integrated明确尚未保存/同步持久任务。

TDD初态合法入口未实现，4项普通测试中3项失败、1项参数拒绝通过，1项原生测试忽略；实现时一次Result::filter编译错误已修正，不抹去失败。最终4项CLI契约及5项既有入口单测通过；显式本机Rust1.98.1原生CLI测试1项通过（3.43秒），缺文档有发现，补齐后零发现，两次都未签发交付或任务关闭。普通命令忽略原生测试，不能将忽略计入通过。相关CLI目标Clippy -D warnings通过，fmt通过。

两份真实机器反馈（缺工具/原生有发现）通过Draft202012 schema；四种伪造authority、delivery_decision、coverage_proven和backlog关闭变体被拒。独立Python协议校验不是新产品运行时。

仍缺原配置/工具运行时及插件闭包核验、完整工作区/features/targets、稳定任务与task verify、抑制对照、check all调度、可信政策与跨平台/三宿主。当前按文件行锚点形成的观察ID尚未作为正式持久任务身份；复杂符号/重排序去重须在任务接线前完善。7.1与完整计划不勾选。

追加真实SIGINT反例先失败：源码变化将取消覆盖为普通退出3。修正为取消优先后，rust_comments_cli最终5项普通通过/1项默认忽略（1.39秒），相关CLI目标Clippy -D warnings退出0（6.54秒）。前述4项是增补取消用例之前的记录，不是最终总数；最终不同用例为5普通CLI、5入口单测和1单独原生补充。完整工作区未重跑，不升级旧880项结果。
