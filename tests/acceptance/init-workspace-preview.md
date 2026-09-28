# `init` 工作区预览与受控创建切片

`codeguard init [path] [--dry-run|--apply] [--format human|json]` 已接入现有只读 `detect`。默认和 `--dry-run` 只返回计划，不创建 `codeguard/`，不执行项目 wrapper、构建、扫描、安装或 Hook。`--apply` 预检查目标，创建用户指定的工作区目录、`.gitignore`、`README.md`、`workspace.json`、`project.json`、`module-graph.json`、`architecture.md` 及根 AGENTS.md 的 Codeguard 受管区块。新文件先写入忽略的 `state/` 暂存区，再以独占硬链接创建目标；已有文件仅在上次工作区摘要证明其仍为受管旧内容时可受控替换，人工改动一律冲突。

项目画像仅保存已静态观察的语言、构建根、声明版本、manifest 摘要、锁文件和配置状态；本机版本为 unknown，架构为 unknown，模块图只写确定的 `contains` 边并注明依赖未解析。生成文件不存本机绝对根路径。工作区受管文件、记录目录及根 AGENTS.md 从普通源码发现中精确排除，`codeguard/src` 用户源码仍被发现；入库安全与普通发现分离。文件身份摘要及计划中的 AGENTS 区块摘要记录在 workspace.json，但不构成批准政策或质量门禁权威。

AGENTS 区块只渲染结构化画像身份、相对链接和当前可用入口，不复制项目自由文本；区块外人工内容与其它工具 marker 按原字节保留。缺失区块时可追加；重复/不配对 marker、改动过的受管内容、符号链接或规划后内容变化返回冲突。写入前再次比对原始字节，并保留已有文件权限；当前尚无能约束所有外部编辑器的强制跨进程锁，最后一次核对与替换之间仍存在竞态，因此 9.21 未完成。

清单、锁文件、旧语言清单精确登记的单文件检查器配置或源码路径集合变化会使 `profile_stale=true`，apply 更新对应画像、AGENTS 摘要并最后更新 `workspace.json`；已有 findings/tasks/decisions 不在受管替换面。配置输入超出有界读取或无法观察时 `observation_complete=false` 且 `profile_stale=null`，不能声称画像新鲜。点前缀例外仅覆盖已登记的配置文件名，其它隐藏源码仍跳过；嵌套点配置目录尚未处理。配置文件被观察到不等于检查器已配置。`package.json` 包版本单列于 `package_declared_versions`，不冒充 JavaScript/TypeScript 语言目标版本。旧版 0.1 工作区可按已记录的画像摘要升级。原始受管投影被人工编辑或工作区身份损坏时拒绝刷新。未登记配置、源码内容和完整依赖关系尚未纳入刷新身份；跨进程编辑器的强制锁也未实现。

该命令当前是**部分实现**：尚未生成初始化准备任务、可信策略映射和完整画像刷新事务。因此 apply 固定返回 `init_status=partial`、退出 3、`readiness=unknown`、`delivery_decision=not_evaluated`；dry-run 返回 `planned`、退出 0 只表示预览成功。公开协议分别见 `schemas/{init-plan,codeguard-workspace,project-profile,module-graph}.schema.json`。

目标测试先因命令缺失失败，AGENTS 增量和刷新用例也先因目标行为缺失失败；随后 `init_command_contract` 覆盖默认只读且不运行 wrapper、受控创建与同输入幂等、清单/锁文件/规则配置/源码路径变化、记录保留、人工文件冲突、受管记录精确排除且用户源码保留、AGENTS 人工及其它受管内容保留、重复/异常/人工改写 marker 与符号链接拒绝、画像/区块摘要绑定与协议边界。新增契约模拟画像先写而 workspace 标记尚旧的中断状态，重试完成且再次运行无改动；新增人工文本于受管区块外，刷新后仍逐字保留。OpenSpec 9.1/9.2/9.15–9.25 仍未完成。

## 2026-09-25 工作区身份绑定

`workspace.json` 0.3 与 init 预览 0.3 加入 `workspace_id`。首次按规范化根路径生成不含明文路径的稳定 ID；刷新沿用旧 ID，两个不同根具有不同 ID。旧 0.1/0.2 工作区按受管摘要升级到 0.3，findings 中的用户记录保留；无效工作区身份仍阻止刷新。`lint python` CLI 反馈 0.4 在已初始化工作区引用该 ID，未初始化、旧版未绑定或损坏工作区明确区分，均不影响固定的 `not_evaluated`。目标 `init_command_contract` 25 项与 `lint_python_cli` 6 项普通测试、固定 Ruff 0.16.8 的 2 项真实 CLI 测试通过。该 ID 不是策略授权；报告持久化和 `work sync` 尚未接线。
