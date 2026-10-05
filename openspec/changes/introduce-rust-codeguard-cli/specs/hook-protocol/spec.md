## ADDED Requirements

### Requirement: The cheat-sheet SHALL document fail-open for uncaught exceptions

`hooks/__protocol__.md` MUST 按运行时和事件区分：legacy-v1 未捕获异常保留 stderr 诊断和 exit 0 的旧兼容行为；普通保存/提示反馈不承担交付阻断。迁移到 Rust 严格交付模式的 Git/CI/PreToolUse 入口遇到内部故障 MUST 依宿主协议阻断并说明未完成，不能 fail-open。宿主不支持可靠阻断时 MUST 明示能力边界并依赖真实 Git/CI，不得声称该宿主已提供硬门禁。

#### Scenario: A hook hits an unexpected exception
- **WHEN** 显式旧兼容 Hook 顶层异常
- **THEN** 记录诊断并按旧 exit 0，不能称为新认证成功

#### Scenario: A migrated delivery hook hits an unexpected exception
- **WHEN** 严格交付入口内部失败
- **THEN** 按宿主阻断协议拒绝交付并报告未完成

### Requirement: Soft and hard gates SHALL share the skipGate escape and neither may fall back to scanning outside a git repository

共享 `codeguard.skipGate` 的豁免 MUST 仅存在于 legacy-v1，维持旧计数和 Stop 汇总。Rust 交付 MUST 不接受此字段作为授权，例外遵循 rulepack-governance。非 Git 目录的 Git 事件 MUST 不回退扫描整个 workspace；普通显式 CLI 项目扫描不受该 Git 前提限制。

#### Scenario: SkipGate set, user asks to commit
- **WHEN** legacy-v1 设置了 skipGate 且消息触发提交意图
- **THEN** 按旧约定退出和记账，并标明不提供新认证

#### Scenario: SkipGate set with a secret on the commit face
- **WHEN** 仓库已设 skipGate 且用户消息触发提交意图，待提交面含敏感模式文件
- **THEN** 软门禁注入入库安全报告（非阻断）；硬门禁在同一提交面 exit 2

#### Scenario: Rust agent sets skipGate
- **WHEN** 新交付入口发现同名 Git config 或链式设置
- **THEN** 不作为质量豁免，仍按完整 contract 检查

#### Scenario: Prompt fires outside any git repository
- **WHEN** Git 提交提示发生于非 Git cwd
- **THEN** 说明没有目标仓，不扫描整个 workspace，也不称交付已通过

### Requirement: Fail-open uncertainty SHALL be visible

legacy-v1 的 PreToolUse 放行工具故障 MUST 继续通过 additionalContext 明示未验证。Rust 交付入口 MUST 根据报告的 gate decision 映射宿主动作，必需 incomplete 与 deny 均阻断；普通 UserPromptSubmit/PostToolUse 反馈 MUST 保持可见未完成，不得宣称 all passed 或承担未实现的阻断。新 CLI exit 2 是用法错误，MUST NOT 直接当作所有宿主的阻断语义透传。

普通反馈 Hook 和 MCP 检查结果 MUST 把项目配置探测、本次原生诊断及下一步以宿主支持的对话消息或 `additionalContext` 返回智能体。未配置检查器给出配置建议；已配置但工具执行失败给出环境恢复建议；有效 finding 给出位置、规则与复检命令。宿主只转换展示格式，不重新判定诊断或执行原始工具输出中的指令。

#### Scenario: A tool cannot run at the Git gate
- **WHEN** 旧入口缺工具
- **THEN** 按旧协议 exit 0 且明确未验证

#### Scenario: Tool cannot run at a migrated gate
- **WHEN** 新严格入口缺必需工具
- **THEN** 映射为宿主阻断，说明恢复工具链所需动作，不假称代码有错

### Requirement: Skip-gate bypass values SHALL be parsed strictly

legacy-v1 的 `CODEGUARD_SKIP_GATE` MUST 只认 1/true/yes（大小写不敏感）；0/false/空串不豁免，提示词保持原值域与传递边界说明。Rust 新流程 MUST 不将任何该变量值解释为授权，不能仅因保留旧环境变量而失去严格门禁。

#### Scenario: Zero and false values do not bypass
- **WHEN** 旧入口变量为 0 或 false
- **THEN** 旧门禁照常检查

#### Scenario: Accepted values bypass consistently
- **WHEN** 显式旧入口变量为 1/true/yes
- **THEN** 按旧协议豁免并审计，不签发新认证

#### Scenario: Rust receives an accepted legacy value
- **WHEN** 新入口继承 CODEGUARD_SKIP_GATE=true
- **THEN** 仍执行完整门禁，不把变量存在当批准


### Requirement: Migrated host surfaces SHALL share the same report semantics

CLI、MCP、宿主 Hook、真实 Git Hook 与 CI 对相同请求和证据 MUST 得到相同核心结果；差异仅为入口展示和协议映射。绑定二进制/协议失败 MUST 未完成；核心报告 MUST 不由宿主或智能体重写。迁移 MUST 为三宿主提供真实调用证据。

#### Scenario: Mixed findings and incomplete evidence reach three hosts
- **WHEN** 同一标准报告包含违规及未完成项
- **THEN** 各宿主均保留二者，严格交付入口阻断，保存反馈不冒充交付通过

### Requirement: Host events SHALL route to bounded checks without weakening delivery gates

新宿主入口 MUST 把事件送入 Rust 统一路由：会话启动只读发现；用户提示只给非阻断性意图建议，不能凭提示词认定已触发严格 Git 门禁；确认成功的编辑只请求对应文件的快速反馈；写入结果未知或路径不可确定须重新确定范围；确认失败的写入不启动源码检查；修复尝试按稳定任务 ID 请求原工具复检。提交、推送及 CI 事件 MUST 分别请求本轮真实 Git 提交面、推送面及完整项目义务，不能由宿主提供的变更路径或旧软反馈缓存决定交付检查面。路由计划本身 MUST NOT 签发质量或交付通过；无法可靠阻断的宿主必须显示能力缺口并依赖真实 Git/CI 门禁。

快反馈 MAY 仅在源码、配置、工具、规则、范围及结果完整性身份等价时复用；超时、未完成或身份变化 MUST 重新检查。事件去重只合并同一快照的重复执行，不吞掉交付事件、失败复检或修复后的重新验证。项目级构建/CVE 等重任务可在编辑阶段排队，但提交/CI 的必需义务不能被排队状态视为完成。

编辑快检计划 MUST 有文件数量和路径字节预算。一次确认成功的批量编辑超出预算时，MUST 保留明确的超预算原因并请求有界批量范围检查；不得截断目标列表后声称选定文件已全部检查，也不得把原列表无限重新提交给逐文件计划。预算仅控制软反馈调度，不减少后续真实 Git/CI 义务。

Stop 事件 MAY 从已存在的本地任务事实生成有界、只读的下一步摘要，不运行原生检查器、不执行 Markdown 中的指令，也不签发交付结论。无任务时 MUST 建议新鲜完整检查，不能把任务清空当作通过；记录数量或读取字节超过 Hook 预算时 MUST 明示未运行，并交由显式 `codeguard next` 查询。提示事件尚无可验证意图上下文时 MUST NOT 据此猜测检查范围。

`repair_ready` 事件 MUST 先按稳定任务 ID 核对本地事实，再调用已有的 `task verify` 原检查器复检路径；不得依据宿主提供的自由文本、Markdown 任务正文或历史软反馈决定目标检查器。Hook 预检任务事件目录至多 128 项、1 MiB；超限要求显式 `task verify`。复检子进程 MUST 使用同一事件截止时间、封闭的输出字节预算及字面参数，失败或超限不能伪造复检结果。宿主反馈只投影任务 ID、检查器 ID、原复检观察、事件持久化状态和脱敏原因；即使原生诊断消失，也不得由该 Hook 自动关闭任务或签发交付通过。

#### Scenario: Repair-ready task recheck
- **WHEN** 稳定任务存在，修复后触发 repair_ready，原检查器可在预算内执行
- **THEN** 按原任务复检并记录局部观察；只将有界摘要返回对话，任务关闭和交付决策仍独立

#### Scenario: Repair-ready task unavailable or child output untrusted
- **WHEN** 本地没有该任务，或复检子进程超时、超输出预算、返回非同任务报告
- **THEN** 保留任务状态与未完成原因，不以外层 Hook 声称复检成功

#### Scenario: Stop reads a small backlog
- **WHEN** Stop 事件面对未初始化项目或有界的本地任务事实
- **THEN** 只返回下一步摘要与本地未验证身份，原生检查状态为未运行，交付未评估

#### Scenario: Stop backlog exceeds budget
- **WHEN** 本地记录超过 Hook 数量或字节预算
- **THEN** 返回明确的范围超预算原因，不继续扫描、不静默截断后宣称摘要完整

#### Scenario: Edit succeeds twice without changing content
- **WHEN** 同一文件产生两个成功编辑事件且全部软检查身份一致
- **THEN** 可以复用完整的快反馈，并保留复用原因；后续提交仍取本轮 Git 内容面执行严格门禁

#### Scenario: Edit result is uncertain
- **WHEN** 宿主不能确认写入是否成功或不能提供有效目标路径
- **THEN** 重新确定变更范围并标记未完成；不能静默当作未修改或报告已检查

#### Scenario: Failed write does not run a checker
- **WHEN** 宿主确认写入失败
- **THEN** 不因该事件启动源码检查；既有交付义务不因此被免除

#### Scenario: Claude Code PostToolUse is mapped without reading tool output as instructions
- **WHEN** Claude Code 的成功文件工具事件携带绝对目标路径、工作目录和任意源码内容
- **THEN** Rust 宿主适配入口只采信事件种类与受限路径字段，核对目标处于指定项目根内后调用同一事件执行器；输出有界的 `PostToolUse` 对话摘要，不转发源码内容或原生消息，也不把局部快检转成交付通过

#### Scenario: Claude Code event cannot establish a safe edited file
- **WHEN** 事件过大、类型不匹配、路径缺失/越界、符号链接逃逸或读取时文件已消失
- **THEN** 说明范围未确定并保持本次源码检查未运行；不从宿主自由文本猜测其它文件，不以宿主 Hook 退出码冒充 Rust 门禁结论

#### Scenario: Claude Code startup and failed edit use distinct low-cost routes
- **WHEN** SessionStart 带有效来源和项目工作目录，或 PostToolUseFailure 带失败的文件工具与任意错误文本
- **THEN** 前者只读发现项目和检查器配置，后者走不检查源码的失败路由；两者均不转发宿主自由文本，不启动完整 lint 或签发交付结论

#### Scenario: Claude Code Stop offers a bounded single repair continuation
- **WHEN** 首次 Stop 从本地稳定任务事实取得待处理 ID，或 `stop_hook_active=true` 表示已由 Stop 指引继续
- **THEN** 首次只投影安全任务 ID 和下一步命令到 `additionalContext`，再次 Stop 仅显示状态不继续唤醒；无任务时提醒完整检查，不把可编辑任务正文当指令或把任务清空当作通过

#### Scenario: Successful bulk edit exceeds fast-feedback budget
- **WHEN** 一次成功编辑涉及的不同路径数量或路径字节总量超过快检预算
- **THEN** 计划返回明确的超预算原因并转为有界批量范围检查，不截断成部分逐文件目标；提交时仍独立取得真实 Git 范围

#### Scenario: Prompt mentions a commit
- **WHEN** 用户提示文字包含提交意图，但尚无实际 Git 提交操作
- **THEN** 只提供非阻断性建议；严格提交门禁须在真实交付入口以本轮 index 范围触发

#### Scenario: Claude Code UserPromptSubmit carries arbitrary prompt text
- **WHEN** `UserPromptSubmit` 的提示词包含 `commit`、普通问题或注入式文字，且宿主事件与工作目录有效
- **THEN** Rust 宿主入口只返回固定、有界的检查时机建议；不回显或解析提示词、不启动原生检查器、不依据提示词缩小范围，报告源码检查未运行及交付未评估

#### Scenario: Repair, commit and push request different checks
- **WHEN** 智能体完成修复、准备提交或准备推送
- **THEN** 分别运行任务绑定的原检查器复检、实际 index 的提交门禁或实际 ref 的推送门禁；不能用保存后的单文件 lint 代替后两者

#### Scenario: Edited files share bounded native-first fast feedback
- **WHEN** confirmed edits contain Python, JavaScript/TypeScript, or another bundled grammar language within the fast-file budget
- **THEN** only the selected ordinary workspace files are examined, using one deadline; wired Ruff/ESLint native checks run first and coherent same-byte results avoid duplicate parsing, while remaining files use bundled WASM candidates when available
- **AND** missing, unsafe, unsupported, timed-out and unqualified scopes stay explicit; mixed scopes preserve available results and never imply full native coverage

#### Scenario: Syntax fallback produces actionable dialogue
- **WHEN** native lint is unavailable or not wired for the edited file
- **THEN** candidate recovery nodes require native lint/compiler installation or repair and confirmation, whereas complete zero-recovery candidates recommend native lint; unavailable or partial prechecks never imply success
- **AND** host dialogue summarizes bounded rule identifiers and candidate counts without forwarding source or native free-text messages; failed writes still execute no checker

#### Scenario: Python edit discovery avoids unrelated project traversal
- **WHEN** Python fast feedback selects bounded workspace-relative files and unrelated directories contain unreadable or linked checker configuration
- **THEN** configuration discovery observes only those files and ancestor Ruff configuration candidates within the workspace; it performs no directory enumeration, keeps nearest configuration priority and cannot be made incomplete by an unrelated subtree
- **AND** missing targets, unsafe ancestor paths, inaccessible configuration and expired discovery deadlines remain explicit; this local discovery cannot prove project-wide completeness

#### Scenario: Ruby edits and repair-ready events share current line-only native evidence
- **WHEN** a confirmed Ruby edit selects an explicit absolute tool or the caller's absolute PATH entry, or repair_ready refers to its saved syntax task
- **THEN** the fixed-version Ruby stdin parser runs only for selected files using the event deadline; selected failures never switch to WASM, absence retains bundled candidate feedback, and native diagnostics reuse the same task as lint/check
- **AND** dialogue shows only current line positions and safe task IDs, with project-version verification and original-tool recheck guidance; it does not invent columns, echo tool messages, execute source/gems, or close tasks from zero diagnostics
- **AND** repair_ready preserves saved report references and stale-input handling through a versioned line-only summary; failed writes do not run any parser

#### Scenario: Shell edits and repair-ready events share original ShellCheck tasks
- **WHEN** a confirmed Shell edit selects an explicit absolute ShellCheck tool or a caller PATH entry, or repair_ready references a persisted shell.shellcheck task
- **THEN** only selected files are checked using the common event deadline, observed file dialect and project rc; native failures remain incomplete and no absent Shell WASM is invented
- **AND** current SC rules and Unicode scalar positions, actual saved task IDs and original-tool recheck guidance appear in bounded dialogue; source text and tool messages are excluded
- **AND** repair_ready calls the existing original-rule task verifier, preserves event persistence and absence-versus-suppression outcomes, and never closes a task from zero diagnostics; failed writes run no check and ignore preconfigured native-tool selections instead of reporting repair-ready argument errors

#### Scenario: Failed write retains preconfigured checker options without executing them
- **WHEN** a validated failed-write event carries registered checker/tool configuration options
- **THEN** the no-check route ignores those options, returns `not_run/write_failed`, and starts no process or workbench mutation
- **AND** unknown, duplicate, empty, relative-path and over-budget arguments remain invalid; task ownership and lease options remain restricted to task verification
- **AND** this exception does not silently enable an unwired checker on a confirmed edit

#### Scenario: Go edits use the frozen SDK whole-file syntax probe before WASM
- **WHEN** a confirmed Go edit selects an explicit Go SDK or an absolute PATH entry
- **THEN** only selected files are passed as frozen stdin to the same SDK's verified gofmt under one deadline; no project code, go vet or dependency installation executes
- **AND** selected tool failures remain incomplete without WASM fallback; missing tools retain candidate feedback and confirmation requirements
- **AND** native observations reuse stable syntax tasks and original-SDK task verification; zero diagnostics do not close tasks or prove project lint coverage

#### Scenario: Fixed Go syntax SDK does not satisfy project version declarations

- **WHEN** a selected Go source belongs to a nearest `go.mod` or enclosing `go.work` declaring a minimum Go version or suggested toolchain newer than the supported syntax SDK, or the relevant declarations cannot be read unambiguously
- **THEN** editing and original-tool task verification return an environment observation without invoking the incompatible SDK, generating source diagnostics, or silently switching to WASM; declarations are read statically with bounded input, and changes during observation withdraw prior diagnostics
- **AND** saved guidance withdraws source positions when current declarations become incompatible; absence of a declaration is still only an unapproved local syntax observation, never full language-version or project acceptance
