# tools install 显式发行清单预览

对应 OpenSpec S05/5.3 局部接线。命令支持唯一 `--distribution-manifest FILE`，仅 install 可用；无此参数继续显示未提供，不自动发现或下载清单。默认 dry-run，apply 仍 blocked_before_mutation。

0.2 反馈有界读取 256 KiB 普通文件，拒绝链接；解析/原始锁绑定复用已验证 API。清单状态为 not_provided/unreadable/invalid/lock_unavailable/unbound/bound_untrusted，固定诊断及字节摘要，不回显原始地址、路径或失败输入。每个工具只显示精确绑定的包格式/摘要/大小/展开上限/源 URI 摘要；声明子集不能证明其它工具或独立运行时完整。authority=unverified、readiness=unknown、writes_performed=false、gate_effect=none、退出 3。旧 0.1 schema 保留为独立文件。

两项新行为测试先因命令不识别参数而失败；实施后工具命令 22 项、发行清单 11 项、crate 边界 4 项共 37 项通过，无 ignored。用例覆盖绑定、apply 仍阻塞、锁原始字节变化、坏清单、链接拒绝、唯一/专用参数及子集不扩大到其它工具/运行时。7 份实际 CLI 输出通过 0.2 schema（未提供、绑定 dry-run/apply、失配、坏清单、不可读、锁不可用），4 个伪造权限/写入/ready/旧版本反例被拒。临时项目未创建 codeguard 目录。

最终 all-target Clippy（-D warnings）、cargo fmt --check、OpenSpec strict 与 git diff --check 均通过。

未进行网络下载、归档展开、可信来源批准或发布；尚无正式 apply、全平台及宿主端到端验收，完整 5.3/5.6 保持未完成。
