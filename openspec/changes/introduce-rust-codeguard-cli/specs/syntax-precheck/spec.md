## Purpose

定义原生 lint 优先、内置 Tree-sitter WASM 语法初检兜底、原生确认与对话修复指引。此能力属于同一 change 的新增目标，当前发布的 0.1.0 未实现。任务唯一状态在 tasks.md 的 S14；初检不替代 comments、类型、dependencies、CVE 或 security 的原生义务。

## ADDED Requirements

### Requirement: Unified lint entry points SHALL select native checking before syntax fallback per module

`codeguard lint java .`、`codeguard lint typescript .` 及 `codeguard check all .` MUST 按模块、语言版本/方言、源码范围、配置与适用原生能力选择执行路径。选择过程 MUST 检查显式工具、项目本地工具及已支持的构建集成，不以 PATH 无命令直接认定未安装。可用原生工具和有效配置 MUST 优先执行；缺工具、配置不可用或原生执行故障可提供内置初检，但 MUST 保留原阻塞及有效原生发现。原生违规 MUST NOT 触发用于覆盖原结论的降级。init/detect/plan MUST 保持只读观察，不因配置发现而启动 WASM 或安装。

只读发现报告的项目本地工具候选 MUST 与检查器配置分开列示；候选只携带相对构建根、检查器、候选状态、可见版本和下一步，不回显可能含凭据的依赖声明。`node_modules` 不进入普通源码范围，固定工具路径可单独有界观察；中间目录或最终入口为链接、特殊文件、不可读或本轮输入变化时 MUST 保留不完整/不可信状态，不跟随链接读取或执行。Maven `.mvn` 配置可作为固定路径观察，不能因普通点目录跳过而误判 Wrapper 缺失。

#### Scenario: Mixed modules have different tool readiness
- **WHEN** 前端具备有效 ESLint 而 Java 模块缺少 JDK
- **THEN** 前端运行原生 lint，Java 模块提供语法初检；两者的范围、来源和准备状态分别保留

#### Scenario: Aggregate checks discover module-local ESLint
- **WHEN** `check all` 发现 JavaScript/TypeScript/TSX 源码及其项目本地受支持 ESLint、单一 flat config 和可用 Node
- **THEN** 在统一任务图和截止时间内逐文件调用既有原生适配器，保留原规则发现；仅对本轮完整原生检查且源码字节未变的文件跳过重复 WASM。缺配置、被忽略、工具故障和其它模块仍分别保留阻塞与降级初检。已初始化工作区串行同步原生任务，重复扫描复用稳定任务；聚合报告、终端和 next 均可取得修复指引。

#### Scenario: Source discovery alone does not establish a required installation
- **WHEN** 聚合原生阶段发现 JS/TS 源码，但未观察到本地 ESLint 上下文
- **THEN** 报告工具上下文缺口，不能仅凭源码存在生成阻断性安装任务或抢占 next；后续准备动作由项目原生义务或语法初检确认需求决定

#### Scenario: Explicit Zig 0.16 source check takes precedence over the candidate grammar
- **WHEN** `lint zig` 收到可核对的 Zig 0.16.0 工具与普通 `.zig` 文件
- **THEN** 优先以受控进程运行原生 `zig ast-check`；原生诊断必须保留，不能因 WASM 观察覆盖。只有该原生工具未提供或不可运行时，才可返回未验收 WASM 候选观察，并始终保持整体未完成；`ast-check` 不等于全部 Zig lint、测试或构建。

#### Scenario: Zig lint and task verification share invoking tool discovery
- **WHEN** Zig lint or a recorded Zig confirmation task is invoked without an explicit tool
- **THEN** select the first ordinary executable zig from absolute invoking PATH directories, ignoring empty/relative entries and non-executable candidates; explicit tools take precedence and a selected failure never switches to another compiler. Bind the selected tool and current source bytes to native observations, retain failure and candidate authority, and do not install tools
- **AND** lint reports selection provenance in a separately versioned feedback; source changes or selected-entry target changes withdraw stale native coordinates, and changed source cannot retain an old-byte WASM observation. A read-only next query does not discover or execute a compiler

#### Scenario: Explicit OTP 28 syntax parsing detects a missing form terminator
- **WHEN** `lint erlang` 收到可核对的 OTP 28 `erl` 工具与普通 `.erl` 或 `.hrl` 文件
- **THEN** 以固定参数和共同截止时间调用原生 `io:scan_erl_form` 与 `erl_parse:parse_form`，读取当前原字节、不展开预处理或执行项目编译插件；检查每项扫描/解析结果，不能把进程正常退出直接解释为零错误
- **AND** 原生结果优先于候选 WASM，缺句点诊断和原生故障分别保留；宏、预处理指令或无法解释的位置保持未完成，不当作已确认源码违规，也不以 WASM 洗白；该阶段只观察单文件 forms，不替代项目 lint、编译或测试
- **AND** 工具未提供时可以附加未验收 WASM 初检，报告其已知缺句点漏检并给出原生确认命令；显式无效工具不能静默改用 PATH 中的同名工具

#### Scenario: A source-file grammar distinguishes function terminators from editor fragments
- **WHEN** Erlang grammar 的原生差分发现源文件函数缺句点、末尾分号或中间缺分隔符被当作零恢复
- **THEN** 修复必须固定原始 grammar、源码模式补丁、真实 external scanner、生成器/编译器版本及原始/派生 WASM 字节；不能通过字符串末尾匹配、伪造 ERROR/MISSING 或忽略差分制造通过
- **AND** 使用同一普通、Unicode/CRLF、注释、字符串/字符/浮点数内句点、多子句、宏与条件编译正反语料，由独立原生工具确认标签；预处理和语言版本的未覆盖范围必须保留，窄语料通过不授予整语言资格

#### Scenario: An Erlang checker is available on the invoking process PATH
- **WHEN** 单文件 `lint erlang` 未指定 `--erl-tool`，且调用进程 PATH 的绝对目录可定位普通可执行 `erl`
- **THEN** 固定所选工具的规范路径，以同一预算核对 OTP 28 版本与工具字节，并优先执行原生 scanner/parser；报告必须区分显式选择、PATH 选择和未找到工具
- **AND** 显式错误工具不能改用 PATH；PATH 首个可执行工具的版本/执行失败也不能被后面的同名工具或 WASM 洗白。空/相对 PATH 条目和不可执行文件不作为自动工具；真正没有工具时保留候选初检与准备指引
- **AND** 工具选择不授予原生完整覆盖或可信权威；单文件无预处理 forms 的范围及交付未评估保持不变

#### Scenario: Aggregate checking observes Erlang forms before candidate parsing
- **WHEN** `check all` discovers ordinary Erlang source files and resolves an explicit `--erl-tool` or an executable `erl` from absolute PATH directories
- **THEN** schedule a bounded native forms task under the shared deadline and jobs limit, retain per-file diagnostics, tool-selection identity, source hashes and actionable original-tool recheck argv in human/JSON/SARIF feedback
- **AND** only current, byte-matching files with complete non-preprocessed native forms observations may skip duplicate WASM; absent tools, unresolved preprocessing, truncated diagnostics, source/tool changes and execution failures remain visible and cannot become native or project success
- **AND** files beyond the native budget remain explicitly unobserved; Java-only selection must not start Erlang, cancellation retains its exit semantics, and useful sibling results remain available. Changed project scope withdraws scope completeness but retains byte-current single-file diagnostics; stale file positions and argv are withdrawn. Forms parsing does not fulfill complete Erlang lint, compilation, tests or trusted task closure

#### Scenario: Task verification finds an already installed Erlang checker
- **WHEN** 已有 Erlang 语法确认或原生首次发现任务执行 `task verify`，未提供显式 `--erl-tool`
- **THEN** 复用扫描入口的工具选择规则，在调用方 PATH 的绝对目录选择首个普通可执行 erl；用共享截止时间核对 OTP 28 版本、工具字节和当前任务源码，记录真实原生复检事件，而不是仅因缺参数报告工具未提供
- **AND** 显式坏工具、所选版本或执行失败不改用后续工具；空、相对或不可执行 PATH 不产生自动工具。任务 next 保留所选原工具的有界复检 argv，后续 PATH 变化不能悄悄替换该明确入口；缺工具仍为环境恢复，不指示源码修改或自动安装
- **AND** 候选与原生首次来源保持原协议和稳定任务身份；repair-ready Hook 复用相同复检入口，局部零诊断不授予可信关闭、完整 lint 或项目通过

#### Scenario: Go vet excludes a file under build constraints
- **WHEN** 本轮 Go 1.23.4 `go vet` 完成，受控 `go list` 证明部分源码进入默认构建，而另一份 `.go` 源码被构建标签排除
- **THEN** 仅对同一源码字节且进入原生包清单的文件跳过重复 WASM；被排除文件继续候选初检，并保持平台/构建标签覆盖未完成

#### Scenario: Clippy completes a current Rust target before candidate parsing
- **WHEN** 聚合检查的锁定离线 Clippy 完成，机器流中的非缓存 compiler-artifact 将目标入口绑定到本轮已观察源码及原根 Cargo 清单
- **THEN** 仅对启动前字节与当前字节一致的该目标入口跳过重复 WASM；终端和 JSON 保留原生发现、原生优先计数及其它文件初检。完整运行无有效 artifact、失败/截断/取消、重复 JSON 键、结束记录后新增事件、工具/输入改变不得产生此覆盖
- **AND** Cargo 的 all-targets、零诊断或 artifact 入口不能证明目录里所有 `.rs` 已参与解析；未被证明的模块、条件排除源码、其它清单目标继续初检，完整 workspace/features/targets、语法资格与交付义务保持未完成。覆盖仅在本次进程内传递，不能从可编辑本地报告导入作为跳过依据

#### Scenario: Cargo is already available on the invoking PATH
- **WHEN** Rust 聚合、comments/build 或原生任务复检没有显式 Cargo 参数，调用进程 PATH 的绝对目录存在普通可执行 Cargo 或 Cargo 代理
- **THEN** 选择首个适用入口，保留 `cargo` 名称运行原生工具；显式无效入口、首个已选择入口的版本/执行失败不能静默改用其它 Cargo。相对或空 PATH 目录、不可执行文件不作为自动工具
- **AND** 子进程保留调用方 RUSTUP_TOOLCHAIN，同时强制 RUSTUP_AUTO_INSTALL=0；项目或调用方工具链缺失应反馈环境未完成，不通过检查隐式下载或安装。工具字节、输入稳定性、锁定离线及原工具复检约束仍适用；发现入口不证明工具/规则批准或完整项目覆盖

#### Scenario: Configured Python root has a local Ruff outside PATH
- **WHEN** 当前受检根配置Ruff且其普通`.venv/bin`目录有可执行Ruff，调用方没有显式工具参数
- **THEN** 原生lint/check及同根任务复检优先选择该入口，然后才考虑调用方绝对PATH；显式参数保持最高优先级，所选工具版本/执行失败不换工具。普通本地目录没有Ruff时可继续PATH；本地目录链接、非目录、损坏/不可执行入口作为具体环境阻塞，不误报源码、不以全局工具掩盖损坏本地环境
- **AND** 无Ruff配置或指定文件没有适用配置时不运行该工具；本轮工具字节、源/config绑定及原生复检约束继续有效。根本地发现不证明嵌套模块不同虚拟环境、其他环境管理器、工具链批准或完整项目覆盖

#### Scenario: A local checker is absent from PATH
- **WHEN** 项目声明的本地原生工具已可按受支持方式定位
- **THEN** 核对其配置和版本后优先运行，不创建错误的安装任务

#### Scenario: Native lint reports a violation or fails after useful output
- **WHEN** 原生工具检出违规，或在有效局部输出后执行失败
- **THEN** 保留原生发现及实际完整性；任何补充初检不能把原结果变成成功

#### Scenario: A read-only planning command observes missing tools
- **WHEN** init/detect/plan 发现工具不可用
- **THEN** 只描述候选后端及准备动作，不执行解析、项目脚本或安装

### Requirement: Syntax precheck outcomes SHALL separate suspected issues from coverage and native execution

版本化报告 MUST 分别描述 backend、precheck、native、scope、observations、setup、persistence/task 引用与 delivery。precheck 状态 MUST 区分 not_run/clean/suspected_issue/incomplete/unsupported；native 状态 MUST 区分 not_run/completed/incomplete 并附原因。疑似观察不等于已确认原生违规。clean MUST 要求非空选定范围全部完成且无异常；有未解析范围时总体 incomplete，同时保留其它有效疑似观察。文件和嵌入区域的已检、排除、失败/不支持计数及原因 MUST 可解释；取消保持取消语义。版本协议及未知 major 的拒绝 MUST 适用于所有消费者。

#### Scenario: No selected files were checked
- **WHEN** 目标为空、全被排除或无适用 grammar
- **THEN** 报告空范围或不支持，不能返回 clean 或完整 lint 通过

#### Scenario: Suspected syntax and a timeout coexist
- **WHEN** 一个文件有 MISSING 节点而另一个文件超时
- **THEN** 总体 incomplete，保留疑似观察与超时文件，不制造已确认源码违规

#### Scenario: An older consumer receives an unknown major
- **WHEN** 报告版本不在消费者支持范围
- **THEN** 拒绝按成功或原生完整结果消费，并返回明确的兼容性诊断

### Requirement: Native setup guidance SHALL distinguish recommendations from required confirmation

缺少可选原生检查器且初检完整正常时 MUST 推荐配置/安装，不创建阻断性任务或使 next 反复选中。初检有疑似异常时 MUST 要求准备适用原生工具并确认。项目已有必需原生义务时，无论初检结果如何 MUST 保留。配置损坏 MUST 指导修复配置，不重复安装已有工具。初检未完成/不支持 MUST 要求恢复有效检查能力而非修改无依据的源码。无适用原生适配器时 MUST 明示能力缺口和具体决策需求，不推荐不存在的工具。安装范围/版本取自项目声明与受控方案，仍受用户既有授权约束。

#### Scenario: Optional checker missing and precheck clean
- **WHEN** 没有既有必需原生义务且初检完整无异常
- **THEN** 返回推荐准备动作；其它修复和 next 不因该建议阻断

#### Scenario: Required checker missing and precheck clean
- **WHEN** 项目已要求该原生检查
- **THEN** 说明必需依据并保留待完成义务，不把它改为推荐

#### Scenario: Suspected issue needs native confirmation
- **WHEN** 初检发现解析恢复异常
- **THEN** 准备/确认成为必需动作，但不能直接把源码定位标为已确认违规

#### Scenario: Setup cannot currently be performed
- **WHEN** 工具安装、配置恢复或适用原生确认能力不可用
- **THEN** 保留一项具体环境/能力决策，不无限重复相同安装动作；独立任务继续

### Requirement: Bundled grammar assets SHALL have pinned provenance and measured compatibility

引入的 CodeGraph 或上游 grammar MUST 固定源码提交、必要补丁、许可证、WASM SHA-256、ABI/运行时兼容范围、语言/方言版本、语料引用及已知限制。活动工作目录中的未固定字节 MUST NOT 直接作为受认可发行输入。文件存在、Rust 加载成功、语法验收完成 MUST 为独立状态。语法包 MUST 随支持平台发行离线可用；不要求用户安装 CodeGraph、tree-sitter-cli 或 Node 解析运行时。Node 可以继续作为 npm 启动层。额外包升级 MUST 经清单校验，不静默使用 latest。

CodeGuard MUST 提供只读的来源覆盖清单，区分 CodeGraph 随仓 WASM、由依赖包提供但尚未固定字节的 grammar、CodeGuard 已复制候选及已验收发行能力。覆盖清单或 `grammar status` 的存在 MUST NOT 自动改变原生检查义务、加载未经批准的字节，或把来源资产数量说成已支持语言数量。来源资产超过当前加载预算时 MUST 明示预算缺口，不能静默跳过。

`grammar status` 的逐语言候选行 MUST 投影固定清单中的已知限制，且只能来自受控、非空、长度有界的文本；已知误报不得被泛化的“待验收”状态掩盖。该投影仍为只读风险提示，不是原生对照、违规确认或白名单决定。

目标范围 MUST 包含当前固定 CodeGraph 来源的 32 份独立 grammar，并包含 Zig；`jsx` 与 `javascript` 共用同一份 grammar，不重复计数。每份 grammar 都须分别完成合法再分发来源、许可、字节与 ABI、Rust 离线加载、版本/方言正反语料、原生工具对照、统一 `lint/check` 的原生优先路由、任务/对话反馈及发行包实装验收。部分候选或来源库存不能充当全部接入。若上游 WASM 与 Rust 运行时导入不兼容，适配 MUST 固定原始和派生字节、限定变换范围并以正反解析样本复验；不能回退到已知产生误报的旧 grammar 来制造可加载状态。

在正式 `lint/check` 路由验收之前，CLI MAY 暴露显式的单文件 `grammar probe` 候选观察。该命令 MUST 通过与正式路径相同的固定资产核验及隔离 worker，报告源码与 grammar 摘要、恢复锚点、未验收状态及原生确认需求；无论有无恢复节点，MUST 返回未完成，不能产生已确认违规、任务关闭或交付通过。有效语种的输入/worker 失败 MUST 使用同一版本化封闭 JSON 协议报告具体未完成原因，未知语种属于参数错误。它不能自动代替已配置的原生工具执行。

#### Scenario: Explicit candidate probe uses one of the 32 assets
- **WHEN** 用户明确对受支持语种和普通 UTF-8 文件执行 `grammar probe`
- **THEN** 固定 WASM 在隔离 worker 中解析该文件并返回候选观察；语法正常仍为未完成，恢复锚点只提示原生确认

#### Scenario: Zig source grammar fixes a known false positive but needs a runtime adapter

- **WHEN** 当前 Zig grammar 能解析合法空容器，而原始 WASM 包含 Rust 运行时不支持的导入
- **THEN** 仅在来源、适配产物及语法反例全部可验证时列为候选；未完成版本语料、原生优先命令和发行验收前仍不宣称 Zig lint 可用

#### Scenario: A Dart source WASM cannot import its external scanner into the Rust worker

- **WHEN** 固定的 CodeGraph Dart WASM 具有旧动态链接元数据和无法解析的外部 scanner 导入
- **THEN** 只可从固定提交的 parser、真实 scanner、头文件和许可证可重复构建新 WASM；固定重建前后字节与限定的导入适配，验证 scanner 相关正反语料；未完成原生对照、路由及发行验收前只能列为候选，不能以空 scanner、旧 WASM 可加载假象或初检无恢复节点宣称通过

#### Scenario: Every CodeGraph grammar is copied but only some are wired to lint

- **WHEN** 32 份资产在来源清单或包内存在，但任一语种缺少真实解析、原生优先路由或可发行验收
- **THEN** 总体覆盖仍未完成，逐语言报告准确缺口，不得把复制数量报告为支持数量

#### Scenario: Source grammar exists but CodeGuard has not qualified it

- **WHEN** 查询固定 CodeGraph 来源中的 grammar 覆盖状态
- **THEN** 列出其来源身份、CodeGuard 集成状态及下一项缺口；不运行解析器、不签发质量或交付通过

#### Scenario: A candidate grammar has a known false positive

- **WHEN** 固定资产清单记录 VB.NET 等语种的具体已知误报
- **THEN** `grammar status` 的该语种行显示有界的已知限制，并继续报告未验收、零发行和原生确认需求

#### Scenario: CodeGraph can load a copied grammar but Rust cannot
- **WHEN** Rust 运行时拒绝 ABI 或外部扫描器组合
- **THEN** 该 grammar 保持不可用并显示兼容原因，不宣称语言初检已支持

#### Scenario: An artifact has no license or pinned source
- **WHEN** 引入清单缺少来源、许可或摘要
- **THEN** 不提升为可发行已验收 grammar，并保留补齐任务

#### Scenario: Fresh installation has no network
- **WHEN** 受支持平台安装包含已验收 grammar 的发行包后离线初检
- **THEN** 加载随包资产完成报告，不下载 grammar 或调用 CodeGraph

### Requirement: Rust parser workers SHALL enforce bounded execution and preserve independent results

Rust runtime MUST 按需加载 grammar，在受控解析工作进程中限制输入字节、诊断数量、内存、并发及总截止时间；worker 不得获得通用文件系统/网络导入。父进程 MUST 在取消/超时/崩溃时终止和回收工作进程，并保留其它模块结果。能力声明 MUST 由宿主实测支持，不能仅凭 WASM 认定隔离成立。运行时选型 MUST 验证 Rust MSRV；改变 MSRV 须明确兼容变更。

#### Scenario: A parser loops or exhausts its memory budget
- **WHEN** 测试 grammar 或输入触发预算上限
- **THEN** 父进程结束该 worker，返回具体未完成原因，兄弟模块结果仍可见

#### Scenario: Cancellation arrives during parsing
- **WHEN** 用户取消初检
- **THEN** 回收工作进程并保留取消状态，不缓存或签发 clean

#### Scenario: A host cannot enforce a declared bound
- **WHEN** 平台无法提供声明的隔离/预算能力
- **THEN** 如实登记缺口，不将该平台标为已通过运行验收

### Requirement: Syntax observations SHALL preserve grammar fidelity and source positions

解析 MUST 同时识别 ERROR/MISSING 恢复，并按同一恢复原因归并级联节点；诊断 MUST 保留原文件位置、必要的有界脱敏上下文及 grammar 身份。不确定版本/方言、模板、宏、嵌入语言 MUST 明示覆盖限制。CodeGraph 用于提取符号的源码遮盖/启发式 MUST NOT 静默移入语法判定。仅凭恢复节点不得猜测具体缺失 token 或自动授予源码修改范围。

若语法树标记错误但可遍历节点中无法定位任何对应 ERROR/MISSING，MUST 将该文件标为初检未完成，并保留原生检查义务；不得将零恢复节点当成语法有效或制造没有位置依据的源码 finding。

同一语言族有独立 grammar 的方言 MUST 按实际文件类型选择和报告；尤其 `.tsx` MUST 使用 TSX grammar，不能拿普通 TypeScript grammar 的恢复节点当作 JSX 源码异常。候选报告读者 MUST 核对 grammar 方言与源文件扩展名一致；不匹配视为报告无效，不生成源码 finding。

多语言共用后缀 MUST 保留歧义：`.m` 可能属于 Objective-C 或 MATLAB，`.sc` 可能属于 Scala 或 SuperCollider。没有可核对的语言证据时，自动检查 MUST 不调用猜测的 grammar、不把恢复节点记为源码问题，并保持范围未完成；用户显式指定 `grammar probe` 的语种仍可作候选诊断。

`.m` 文件中 MATLAB 块注释 `%{ ... %}` 内的行首 `#import` 或 `@interface` 只是注释数据，MUST NOT 作为 Objective-C 证据；块注释外的明确 Objective-C 专属标记仍可用于候选路由。发现与检查 MUST 使用同一判别，避免一个入口报告未知而另一个入口执行错误 grammar。

#### Scenario: Shared extension would produce a speculative syntax finding
- **WHEN** 项目包含无 Objective-C 专属标记的 MATLAB `.m` 源码，或 SuperCollider `.sc` 源码
- **THEN** 自动候选路由不把它们交给 Objective-C/Scala WASM；歧义范围保持未完成，不能用资产数量或其它文件的检查结果代替该范围

#### Scenario: An Objective-C marker appears only in a MATLAB block comment

- **WHEN** `.m` 文件的 `%{ ... %}` 块注释内含行首 `#import`，块注释外只有 MATLAB 语句
- **THEN** `detect` 与 `check all` 均保持该文件为未解析的歧义范围，不调用 Objective-C grammar；块注释外的真实 Objective-C 标记仍可路由

#### Scenario: Valid JSX is parsed with a TypeScript-only grammar

- **WHEN** `.tsx` 文件含合法 JSX，而普通 TypeScript grammar 会产生恢复节点
- **THEN** 选用固定的 TSX grammar 并保留候选未验收状态；不得制造疑似源码异常或声称原生 lint 已执行

#### Scenario: One unmatched delimiter produces many recovery nodes
- **WHEN** 同一恢复原因造成重复或级联节点
- **THEN** 归并为可处理观察并保留受影响范围，不创建大量同义任务

#### Scenario: Grammar hides a missing token from node traversal

- **WHEN** grammar 的根节点 `has_error=true` 且 S-expression 含缺失 token，但运行时的可遍历子节点不能定位该 token
- **THEN** 初检标为 incomplete，保留未完成计数并请求原生确认；零恢复节点不得升级为 clean
- **AND** 已初始化工作区的 check/确认写入 Hook 将这种已绑定源码与 grammar 的观察同步为同一稳定检查恢复任务，保留无位置证据及检查未完成原因；next、可读任务和对话上下文要求恢复适用原生能力或调查 grammar，原生确认前不授予源码修改范围。重复扫描不新增同义任务，完整零恢复观察不新建该任务、不关闭已有任务；保存失败保留原报告且不返回虚假任务 ID
- **AND** 没有该语言原生确认 adapter 时，task verify 记录具体能力缺口，任务保持开放，不能换用其它语言工具或伪造关闭；未知报告版本、零恢复且无未完成原因、错源码/grammar 身份或虚假定位均不可导入

#### Scenario: A template includes unsupported embedded syntax
- **WHEN** 只支持文件中一部分语言区域
- **THEN** 通过位置映射返回已检/未覆盖区域，不报告整个文件 clean

#### Scenario: A CFQuery tag appears inside different comment forms
- **WHEN** CFML 文件的可嵌套 `<!--- ... --->` 注释、普通 HTML 注释和标记正文中各含完整的 `<cfquery>...</cfquery>` 标签
- **THEN** 跳过被 ColdFusion 移除的 CFML 注释内标签，保留仍由 ColdFusion 处理的 HTML 注释内标签及正文标签；候选整体仍不获得原生 lint 或交付权威

#### Scenario: A CFML comment appears inside a query opening tag
- **WHEN** 合法 `<cfquery>` 开标签的属性之间含可嵌套 CFML 注释，其文本带有 `>`
- **THEN** 跳过注释后定位真正的开标签结束位置，查询体范围与原文件字节偏移保持一致

#### Scenario: A newer dialect is outside the validated range
- **WHEN** 文件使用未验收语言版本或方言
- **THEN** 保留兼容性限制和原生确认动作，不直接断言合法源码违规

#### Scenario: A candidate grammar matches a narrow native syntax corpus
- **WHEN** 同一组合法与破损源码分别经过固定版本的语言原生语法工具和隔离 WASM worker
- **THEN** 逐例记录语法分类一致性、工具与语料身份；仅扩大该版本的局部精度证据，不将候选提升为完整 lint、其它版本或交付通过

#### Scenario: Native syntax rejects a source that the WASM candidate accepts
- **WHEN** 固定 OTP 28 编译器拒绝函数缺少最终句点的源码，而固定 Erlang WASM 完整恢复扫描没有节点
- **THEN** 记录带原生证据的漏检及待修复 grammar 身份，保持候选未验收和完整检查义务；不能因零恢复节点签发 clean、放入白名单或关闭任务

#### Scenario: Differential evaluation receives an incomplete zero-diagnostic scan
- **WHEN** Swift 或 Kotlin 固定样例返回空恢复数组，但隐藏错误使扫描未完成
- **THEN** 该样例保留在总语料分母并单独计为未知覆盖；不能计作合法、非法、一致、误报或漏报，且不能删除该样例来提高精度指标

#### Scenario: All bundled grammars are replayed for development evaluation
- **WHEN** 开发期 Rust 回放入口消费绑定当前清单摘要、源码摘要及来源的固定回归语料
- **THEN** 逐份固定 grammar 调用现有隔离 worker，输出每语言的全样本数、可判定分类、未知、TP/FP/FN、区间和有界耗时；重复键、语言缺项、字节身份失配在启动前拒绝。回归标签和待裁定标签分开，待裁定样本不进入精度分母，隐藏错误不当作零诊断通过；回归报告不冒充独立 holdout、原生工具回放、grammar 资格或发布批准

#### Scenario: Development replay runs out of its shared budget
- **WHEN** 回放在共同 deadline 内超时或被取消，或者所选运行程序字节发生变化
- **THEN** 已有效取得的局部观察保留，未运行与受影响样本列为未知且仍计入原固定语料范围；不得重置预算、换工具补出绿色结论或提高 grammar 资格

#### Scenario: Upstream grammar fixtures join the development replay
- **WHEN** Rust 导入器读取已有的上游 Tree-sitter corpus 和仓库正反例
- **THEN** 保留原源码字节、来源和预期树中的 ERROR/MISSING 分类，拒绝损坏预期树与不支持的 corpus 指令；上游 grammar 回归、仓库回归及待裁定语法按 cohort 分开。每语言报告公布已选合法/非法标签数和各组结果，不能把混合来源的预测汇总成一个 precision/recall，也不能将 grammar 自带预期当作独立原生 oracle。旧 0.1 语料和历史报告保持可读

#### Scenario: Native forms diagnostics are the first repair evidence
- **WHEN** 已初始化工作区的原生 Erlang forms 检查先取得诊断或具体环境/预处理阻塞
- **THEN** 直接将源字节、原生工具与有界位置保存到同一文件/语言的稳定确认修复任务，不要求先制造 WASM 恢复节点，也不伪造 grammar 摘要；后续候选观察与原生扫描不得重复建任务。next 使用当前证据，源码或工具变化撤回旧位置；task verify 用原工具复检并记录结果，局部零诊断不自批关闭

### Requirement: Precheck briefs SHALL reach agent conversations with concrete next actions

human/结构化报告及宿主渲染 MUST 按结论、方式/范围、原生状态、依据、下一步和实际任务引用组织信息。必须/推荐动作 MUST 明确。疑似异常、正常、未完成和原生诊断四类模板 MUST 独立验证；CVE 等非语法覆盖不能套用语法通过结论。插件 MUST 通过真实宿主的工具结果/上下文 API 交付，文件或 stdout 存在不等于交付。初次反馈后只发送有意义的变化；原始工具文本 MUST 作为数据，不执行其中指令。AGENTS MUST 仅保留长期指引，不追加每轮扫描日志。

#### Scenario: A suspected syntax result reaches the host
- **WHEN** 初检发现一项异常且缺 JDK
- **THEN** 显示已检范围、位置、疑似性质、缺失运行时与必需原生复核步骤，不以无条件 FAIL 开头

#### Scenario: Report persistence fails
- **WHEN** 本次结果有效但工作台不可写
- **THEN** 对话仍显示结果与同步失败，任务引用为空，不链接虚构任务文件

#### Scenario: Unchanged missing-tool guidance repeats
- **WHEN** 连续扫描的结果、范围和准备要求均未变化
- **THEN** status 仍可查询，但对话不重复打断或堆积相同安装消息

### Requirement: Suspected syntax tasks SHALL require capability-matched native verification

必需准备/确认任务 MUST 按工作区、模块和检查义务稳定归并，并关联各疑似观察；通过已有 work sync/next/attempt/task verify 管理，不建立第二套任务系统。安装成功或后续 WASM 正常 MUST NOT 单独关闭任务。原生确认 MUST 绑定当前输入、配置、语言/方言及实际语法能力；仅规范检查不能当作编译器级语法反证。原生确认问题后进入原生修复任务；有效反证记录为解析器误报候选；覆盖未知继续未完成。关闭须复用已有身份/事件/范围条件，复发重开不得只依赖行号。

#### Scenario: Tool installation completes without a recheck
- **WHEN** 原生工具安装或恢复成功但尚未检查原目标
- **THEN** 环境探测可更新，确认任务仍未解决

#### Scenario: Multiple edited grammar candidates need native confirmation
- **WHEN** initialized workspace edit feedback contains bounded recovery evidence for one or more bundled grammar languages
- **THEN** import a versioned, source-and-grammar-bound observation into the existing task store, keeping one confirmation identity per workspace, file and language; Python and ESLint scopes reuse their existing preparation identities
- **AND** preserve raw byte locations as suspected evidence, never source violations; complete zero-recovery observations create no new mandatory task and cannot close previous tasks
- **AND** unavailable native confirmation adapters yield a concrete pending capability decision, never a Python fallback command for another language; persistence failure preserves feedback and exposes no fictitious task reference

#### Scenario: Style-only checker returns zero diagnostics
- **WHEN** 该检查器不能确认疑似语法所需能力
- **THEN** 不关闭任务、不判 grammar 误报，指出所需的适用确认工具

#### Scenario: A Zig confirmation task replays the native AST checker
- **WHEN** task verify receives an explicit applicable Zig 0.16.0 tool for a recorded Zig candidate scope
- **THEN** feed the current bounded source bytes to native ast-check under the existing task lease and deadline, bind the source/tool identities, and retain the observation and attempt association
- **AND** distinguish native diagnostics, zero diagnostics, unavailable tools and changed inputs; zero diagnostics alone cannot grant formal resolution or project delivery

#### Scenario: A fresh confirmation task already has an applicable native adapter
- **WHEN** a verified original candidate report identifies Zig, Erlang or Swift and no native recheck has yet been recorded
- **THEN** next and task show describe the actual available adapter and its supported version, distinguish unknown local tool readiness from an unavailable adapter, and provide the applicable native tool selection argument
- **AND** the read-only query does not run or install a tool, does not take executable paths from editable task text or historical candidate data, and does not authorize source repair or task closure before native confirmation
- **AND** languages without an implemented confirmation adapter retain the concrete capability decision; zero WASM recovery does not remove the obligation
- **AND** structured preparation guidance uses a versioned contract that binds language, supported version, tool selection parameter and explicitly unassessed local readiness; it does not invent a native execution status or evidence reference, and aggregate consumers accept that exact version while historical schemas remain unchanged

#### Scenario: A native syntax confirmation observation becomes stale
- **WHEN** the source or tool changes after a recorded native syntax observation
- **THEN** next and task show require a fresh native confirmation and preserve historical evidence; they cannot recommend a stale source repair or pretend the old zero-diagnostic observation still applies

#### Scenario: An Erlang confirmation task uses its native forms parser
- **WHEN** task verify or repair_ready receives an explicit OTP 28 erl tool for an existing Erlang candidate task
- **THEN** run the same controlled native scanner/parser as lint erlang on the current task scope, under the existing lease, attempt association and shared deadline
- **AND** retain versioned source/tool-bound observations and native positions; next provides reusable --erl-tool argv only while the tool identity is current
- **AND** preprocessing, unknown output, tool failure and input changes remain incomplete with concrete reasons; native zero diagnostics alone cannot close the task, and Erlang options on another language are rejected before acquiring a lease or starting a tool

#### Scenario: Native syntax confirmation contradicts the parser
- **WHEN** 当前同输入、范围和方言的适用原生检查完整正常
- **THEN** 记录限定范围反证和解析器误报调查；不自动扩大白名单

#### Scenario: Source changes or task files are removed
- **WHEN** 复检前输入变化或智能体删除/勾选任务文件
- **THEN** 旧观察不作为当前关闭依据，真实检查义务不因任务文件操作消失

#### Scenario: The agent repeats native confirmation through repair-ready feedback
- **WHEN** 已初始化任务的 Zig 原生观察已记录可用工具路径，智能体修复后触发 repair_ready
- **THEN** next MUST 提供可复用的 task verify argv，Hook MUST 接受同任务的显式 Zig 工具并复用原有租约、尝试和共享截止时间；错误语言、陈旧字节或工具身份不得使旧诊断成为当前修复依据

### Requirement: Grammar false-positive dispositions SHALL be precise and preserve native obligations

白名单 MUST 绑定规则/恢复类型、目标或稳定源码锚点、grammar 版本/摘要、原因及既有独立批准条件。原始观察及人工裁定 MUST 保留；版本、范围或相关输入变化须复核。白名单只改变指定误判处置，MUST NOT 补全覆盖、关闭必需原生确认或将工具故障当误报。系统性误报 MUST 转为 grammar/适配器修复与正反语料任务，不能自动扩大文件排除。

#### Scenario: Exact approved disposition matches
- **WHEN** 同一版本、规则、目标满足有效处置
- **THEN** 展示命中理由并保留原始观察，仍保留必需原生检查

#### Scenario: Grammar upgrades or agent edits approval locally
- **WHEN** 资产身份变化或 agent 自写 approved
- **THEN** 旧处置重新评估或不生效，不将其变成普通通过

### Requirement: Syntax caches SHALL bind inputs and retain historical observations on invalidation

缓存 MUST 绑定源码字节、语言/方言画像、grammar/runtime、规则/查询版本及相关范围/配置身份。部分结果、取消、未支持范围 MUST NOT 作为 clean 重用。资产更新/回滚须核对版本清单并使相关缓存与处置失效；删除缓存不能删除问题历史或签发解决。相同 mtime/大小不构成身份相同。

编译入同一二进制的不可变清单、WASM 与许可证 MAY 在进程内复用已经完整核验的资产结果；外部清单、源码、配置与原生工具 MUST NOT 因此省略当前身份复核，此资产复用不得作为检查结果或完整覆盖缓存。

#### Scenario: Source changes without mtime or size change
- **WHEN** 源码字节不同但时间和大小相同
- **THEN** 缓存失效并重新检查

#### Scenario: Parser pack is rolled back
- **WHEN** 选择已验证的较旧清单
- **THEN** 重建受影响解析结果，保留旧观察与任务历史，不自动关闭异常

### Requirement: Fallback rollout SHALL preserve command and delivery compatibility

WASM fallback MUST 通过明确版本的协议加入既有统一命令，不静默重定义退出码。未完成必需原生检查仍为 3，取消130、内部故障4、用法2保持契约；疑似解析异常不直接作为已确认违规1。lint/check 的初检正常不能冒充原生成功或交付许可。专用 syntax 命令若以后新增 MUST 另行定义范围，本能力不依赖该新命令。文档、help、schema、插件消费者和发行说明 MUST 明示当前/目标能力；历史报告不得被改写成新版已完成。

#### Scenario: A CI caller only consumes exit status
- **WHEN** 必需原生检查缺失但 WASM 初检正常
- **THEN** 返回未完成3，不把原生义务降为成功0

#### Scenario: Published package predates fallback support
- **WHEN** 用户安装不含此能力的版本
- **THEN** 文档与支持矩阵不宣称自动初检已可用，不用设计示例冒充运行记录

#### Scenario: Swift hidden recovery receives native parse confirmation
- **WHEN** 已消费 Swift WASM 确认任务包含无法定位的恢复异常，调用方提供已安装的 Apple Swift 6.4 原生编译器绝对入口
- **THEN** Rust 以冻结 stdin、固定非项目 cwd 和共同截止时间调用 frontend parse；不构建、类型检查、加载项目插件或运行源码
- **AND** 只把可核对 UTF-8 字节边界的原生错误位置写入同一任务的追加历史；next 和 repair_ready 返回脱敏原生位置、字节列单位和实际复检入口
- **AND** 缺工具、版本不匹配、异常输出、超时及输入变化保留未完成；零诊断仅表示局部语法未观察到问题，不自动关闭或替代完整 lint、构建和交付

### Requirement: Syntax support claims SHALL have per-language and per-host evaluation evidence

验收 MUST 分别记录合法/非法语料、ERROR/MISSING、语言版本/方言、模板位置映射、原生对照、误报/漏报、未知覆盖及白名单处置。每个声明支持的语言/宿主 MUST 有独立证据；CodeGraph 的支持清单不自动继承为 Codeguard 能力。冷/热启动、包大小、内存/并发及异常退出预算须实测后固化，不编造精度或性能数值。三个宿主的实际对话交付应分别验证，任一个通过不能替代另两个。

#### Scenario: Grammar corpus passes but native confirmation is absent
- **WHEN** 只有解析器样例成功
- **THEN** 仅记录对应解析能力，不宣称原生闭环、全语言或宿主验收完成

#### Scenario: False positives disappear through exclusions
- **WHEN** 扩大排除导致表面误报数下降
- **THEN** 评测仍保留原始分母、排除和未完成覆盖，不能据此声称准确率提高

#### Scenario: Kotlin native confirmation distinguishes syntax from missing project context
- **WHEN** an explicit installed Kotlin/JVM 2.4.10 compiler checks a frozen ordinary `.kt` file in a private directory
- **THEN** invoke its native compilation command without project build scripts, response files, compiler plugins or script execution; bind current source bytes and retain bounded compiler diagnostics
- **AND** classify `[SYNTAX]` as native syntax evidence while unresolved references and other semantic diagnostics remain project-context limitations, not fabricated syntax violations; preserve syntax evidence when both coexist
- **AND** only accept diagnostics belonging to the frozen input, translate verified native UTF-16 columns to UTF-8 byte positions, and reject unknown output, contradictory exit codes, truncated output, timeouts and changed input/tool identities
- **AND** this single-file observation does not implement full Kotlin lint/comments, prove whole-project coverage, qualify its WASM or authorize trusted task closure; missing tools require preparation, never implicit installation

#### Scenario: Kotlin selects native tooling before candidate fallback
- **WHEN** a caller supplies an explicit compiler or invoking absolute PATH contains `kotlinc`
- **THEN** retain that first selected tool and its failures without silently replacing it; only tool absence enables candidate WASM
- **AND** missing backends, incomplete native checks or candidate recoveries/hidden errors require further native confirmation; complete observable candidate zero-recovery may recommend native installation without granting delivery
- **AND** human feedback includes verified native rule and source positions, while JSON preserves bounded evidence and unresolved obligations

#### Scenario: Existing Kotlin confirmation task uses the native compiler
- **WHEN** task verify or repair_ready targets a bound Kotlin confirmation task
- **THEN** use the same explicit/PATH native-tool selection and frozen-source observation as lint, append verification evidence to the existing stable task and preserve lease/attempt handling
- **AND** reject wrong-language or relative tool arguments before acquiring a lease or starting a process
- **AND** expose current syntax positions separately from unresolved project-context diagnostics, including mixed observations; changed source/tool identities withdraw stale positions
- **AND** native zero diagnostics does not by itself prove policy/coverage or close the task; initial tool guidance must preserve the Kotlin selection option in task show and next

#### Scenario: Kotlin first observation prefers a selected native compiler
- **WHEN** check all or a confirmed file_changed event observes ordinary Kotlin files
- **THEN** invoke the selected explicit or first absolute-PATH compiler before candidate parsing, within the shared deadline and requested scope
- **AND** retain native syntax and context observations in a stable task without fabricating a WASM origin; selected-tool failure must not switch to a candidate parser
- **AND** tool absence enables WASM, complete native zero diagnostics creates no new task, and repeated scans reuse existing identity and current evidence

#### Scenario: Incomplete native history retains unlocated recovery guidance
- **WHEN** a stable task originated from incomplete or unlocated candidate recovery and a later native observation still cannot confirm source diagnostics
- **THEN** next and task show retain the unlocated limitation, forbid source edits before native confirmation, and provide concrete native-environment recovery guidance; a missing tool must not erase the original limitation or fabricate positions

#### Scenario: Swift standalone checking selects native parsing before candidates
- **WHEN** lint swift receives an ordinary Swift file and an explicit compiler or the first executable swiftc in absolute invoking PATH
- **THEN** reuse bounded frozen-source frontend parsing, preserve validated UTF-8 byte positions, and retain selected-tool failure without selecting another compiler or WASM
- **AND** only tool absence enables bundled candidate parsing; incomplete/hidden recovery requires native confirmation, complete zero recovery recommends native setup without granting project lint or delivery success

#### Scenario: Swift project observation preserves native preference and scope gaps
- **WHEN** `check all` discovers ordinary Swift sources
- **THEN** it selects the explicit Swift tool or first executable invoking PATH entry before candidates, parses at most 64 frozen files under the shared deadline, reports unobserved sources and current byte positions, and never changes a selected failing compiler into a WASM success
- **AND** missing tools retain candidate fallback, source or tool changes withdraw positions, and project lint, task connection and delivery gaps remain visible until independently implemented and verified.

#### Scenario: Confirmed Swift edits provide native evidence to the host
- **WHEN** a confirmed `file_changed` event selects Swift sources
- **THEN** the Hook uses the same explicit/PATH native parser selection within the shared deadline and checks only selected sources, without replacing selected native failures with candidates
- **AND** bounded host feedback includes current native counts and byte positions, never raw tool messages, and explicitly preserves missing task connection and full-project obligations.

#### Scenario: Swift native first observations join a stable repair task
- **WHEN** an initialized workspace observes current Swift native diagnostics or a selected-tool blocker
- **THEN** the project/save scan records native-first evidence without fabricated grammar identity, updates one stable source-scope task, provides current repair/environment guidance and supports original-parser task verification
- **AND** a subsequent clean observation records evidence but cannot auto-close; absent tools still use candidate confirmation, and failed persistence retains diagnostics with an explicit task gap.

#### Scenario: Aggregate checks and edit feedback prefer the invoking Zig checker
- **WHEN** check all, check zig or selected-file editing observes ordinary Zig files and an explicit or invoking-PATH Zig tool
- **THEN** reuse the frozen native AST probe under the shared deadline; report original diagnostics, selected-tool failures, changed input and unobserved files without substituting WASM after a selected-tool failure. Missing tools retain candidate fallback and preparation guidance. Project lint/build and trusted closure remain independent obligations; no native diagnostic may disappear because the candidate grammar reports no recovery.

#### Scenario: First-native Zig observations retain one actionable confirmation task
- **WHEN** an initialized workspace receives a current Zig AST diagnostic or selected-tool failure from lint, aggregate checking or a confirmed edit
- **THEN** import a language-versioned native-first report with no grammar identity into the existing stable task, synchronize once per bounded scan, retain original-tool guidance and real report references, and reuse the same ID on repeated observations. A new complete clean observation creates no task; an existing task records the observation but requires independently authorized capability-matching verification before closure.


#### Scenario: Zig native output cannot create false completion or invalid repair coordinates
- **WHEN** the Zig AST process exits zero with unexpected stdout, or a diagnostic position exceeds the frozen source line/byte bounds
- **THEN** its observation remains incomplete and invalid positions are withdrawn; neither a clean result nor actionable coordinates may be inferred from the exit code alone


#### Scenario: Explicit native grammar differential retains unknown and unselected coverage
- **WHEN** the development evaluator compares frozen corpus bytes through explicitly installed native tools and the existing WASM worker
- **THEN** reject invalid selections before process execution, retain all 32 languages in inventory, compare only jointly decidable syntax samples, keep context-only native blockers or hidden WASM recovery as unknown, preserve strictly located Kotlin syntax diagnostics even when mixed context blockers leave execution incomplete, withdraw classifications after tool/program identity changes, and do not promote reused-adapter regression evidence to independent holdout, grammar qualification or delivery permission

#### Scenario: Python native grammar comparison excludes ordinary lint and project configuration
- **WHEN** an explicit installed Ruff is used as the Python syntax observer in development native/WASM comparison
- **THEN** freeze the tool/version and target dialect, feed the same source through isolated stdin with fixes/cache/project configuration and noqa suppression disabled, classify only consistent located `invalid-syntax` reports, retain other rules, wrong-source reports, tool failure or contradictory exit/JSON as incomplete, and do not extend trusted task-closing or project-delivery authority

#### Scenario: Structural empty blocks remain distinct from parser recoveries
- **WHEN** an application scans a grammar tree for empty `block` nodes that contain no non-comment named statement
- **THEN** traverse even branches without `has_error`, retain the direct parent kind and original byte positions with explicit record/traversal budget exhaustion, and do not label these structural facts as parser ERROR/MISSING or language violations; legal empty blocks in other languages require independent language interpretation
