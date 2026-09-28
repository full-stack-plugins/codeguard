# tools install 未批准候选预览

对应 OpenSpec 5.3/5.6 局部实施；正式可信安装未完成。

`tools install --lock FILE [path] [--dry-run|--apply] [--managed-cache ABS_PATH] [--runtime ID=ABS_PATH] [--format human|json]` 默认 dry-run，显式锁必需，模式不可重复或冲突。共用已有锁与制品静态核验，不执行/下载/安装或写工作区。

预览绑定锁原始摘要、精确工具二进制摘要及来源引用摘要，列出声明、平台、runtime/bundle 缺口、恢复动作和缺失的批准锁/发行清单/平台包格式。来源引用不回显宿主路径。已有匹配制品仍需批准工具链核验；其它平台不适用，项目 wrapper 与系统工具分别提供恢复/授权计划动作。

当前没有可信安装来源，apply_requested 明确 blocked_before_mutation；writes_performed=false、readiness=unknown、authority=unverified、gate_effect=none，退出 3。不调用缓存发布 API，也不将候选摘要匹配解释为授权。

三个入口用例先失败，实施后工具 CLI 18、身份 8、配置 6 共 32 项通过。三份实际 dry-run/apply 输出通过 schema；两种模式冲突退出 2；四项伪造写入/批准/安装成功/ready 被拒。all-target Clippy（-D warnings）、格式、OpenSpec strict/diff 检查通过。

完整可信下载清单、源定位披露/批准、发行包展开、正式 apply/恢复及平台/宿主端到端尚缺；未运行全 workspace，不勾选完整任务。
