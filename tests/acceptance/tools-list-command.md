# tools list 声明库存与静态制品缺口

对应 OpenSpec introduce-rust-codeguard-cli/unified-cli-contract、5.6 局部实施。

## 命令与观察

`codeguard tools list [path] [--tool-lock-candidate FILE] [--managed-cache ABS_PATH] [--runtime ID=ABS_PATH] [--format human|json]`

默认锁为 codeguard.lock.json；不要求初始化工作区，不保存报告、不启动 wrapper、联网或安装。复用 tools verify 的有界普通文件读取、严格锁协议和三来源制品核验，公开输出不包含本机路径或原始来源引用。

tool_inventory_observation 0.1 显示全部声明平台并按工具 ID/平台稳定排序：

- 声明版本、运行时、adapter 和 native_builtin/external_rulepack 来源，与制品观察分开。
- 当前平台核对 tool/runtime/bundle；字节与可执行位匹配仅 matched_untrusted，缺口保留具体 issue。
- 其它平台为 other_platform/not_inspected，三个制品观察为空，不访问该平台文件或推导本机缺失。
- 范围固定 declared_lock_only、required_inventory unverified、required_by_policy null；候选锁结构正确不能成为必需策略、可启动证明或 ready。
- 缺锁、坏锁或符号链接锁明确反馈；不把空库存当作无必需工具。局部观察固定退出 3，help 明示 list/verify。

## 验证

最初四项新增用例因 tools list 无入口/无 JSON 输出失败；实现后继续增加自写 required_by_policy 拒绝、排序及 external_rulepack 声明用例。

tools_verify_cli 14 项、tool_identity_contract 8 项、config_inspection_contract 6 项共 28 项通过；既有 verify 反馈契约保留。wrapper 带副作用但未执行，目录没有自动初始化。

五份实际库存输出（缺锁、项目 wrapper、系统制品、受管缓存缺失、其它平台）通过新 schema；五项伪造 authority、readiness、gate、完整必需库存及 required_by_policy 被拒绝。all-target Clippy（-D warnings）、格式、OpenSpec strict 与 diff 检查通过。

尚无受批准必需/可选集合、完整可用库存、显式安装和受管发行恢复；不勾选 5.6。未运行全 workspace 测试、原生启动或宿主端到端。
