## Purpose

定义原生 lint 优先、内置 Tree-sitter WASM 语法初检兜底、原生确认与对话修复指引。此能力属于同一 change 的新增目标，当前发布的 0.1.0 未实现。任务唯一状态在 tasks.md 的 S14；初检不替代 comments、类型、dependencies、CVE 或 security 的原生义务。

## ADDED Requirements

### Requirement: Unified lint entry points SHALL select native checking before syntax fallback per module

`codeguard lint java .`、`codeguard lint typescript .` 及 `codeguard check all .` MUST 按模块、语言版本/方言、源码范围、配置与适用原生能力选择执行路径。选择过程 MUST 检查显式工具、项目本地工具及已支持的构建集成，不以 PATH 无命令直接认定未安装。可用原生工具和有效配置 MUST 优先执行；缺工具、配置不可用或原生执行故障可提供内置初检，但 MUST 保留原阻塞及有效原生发现。原生违规 MUST NOT 触发用于覆盖原结论的降级。init/detect/plan MUST 保持只读观察，不因配置发现而启动 WASM 或安装。

只读发现报告的项目本地工具候选 MUST 与检查器配置分开列示；候选只携带相对构建根、检查器、候选状态、可见版本和下一步，不回显可能含凭据的依赖声明。`node_modules` 不进入普通源码范围，固定工具路径可单独有界观察；中间目录或最终入口为链接、特殊文件、不可读或本轮输入变化时 MUST 保留不完整/不可信状态，不跟随链接读取或执行。Maven `.mvn` 配置可作为固定路径观察，不能因普通点目录跳过而误判 Wrapper 缺失。

#### Scenario: Mixed modules have different tool readiness
- **WHEN** 前端具备有效 ESLint 而 Java 模块缺少 JDK
- **THEN** 前端运行原生 lint，Java 模块提供语法初检；两者的范围、来源和准备状态分别保留

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

#### Scenario: Source grammar exists but CodeGuard has not qualified it

- **WHEN** 查询固定 CodeGraph 来源中的 grammar 覆盖状态
- **THEN** 列出其来源身份、CodeGuard 集成状态及下一项缺口；不运行解析器、不签发质量或交付通过

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

#### Scenario: One unmatched delimiter produces many recovery nodes
- **WHEN** 同一恢复原因造成重复或级联节点
- **THEN** 归并为可处理观察并保留受影响范围，不创建大量同义任务

#### Scenario: A template includes unsupported embedded syntax
- **WHEN** 只支持文件中一部分语言区域
- **THEN** 通过位置映射返回已检/未覆盖区域，不报告整个文件 clean

#### Scenario: A newer dialect is outside the validated range
- **WHEN** 文件使用未验收语言版本或方言
- **THEN** 保留兼容性限制和原生确认动作，不直接断言合法源码违规

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

#### Scenario: Style-only checker returns zero diagnostics
- **WHEN** 该检查器不能确认疑似语法所需能力
- **THEN** 不关闭任务、不判 grammar 误报，指出所需的适用确认工具

#### Scenario: Native syntax confirmation contradicts the parser
- **WHEN** 当前同输入、范围和方言的适用原生检查完整正常
- **THEN** 记录限定范围反证和解析器误报调查；不自动扩大白名单

#### Scenario: Source changes or task files are removed
- **WHEN** 复检前输入变化或智能体删除/勾选任务文件
- **THEN** 旧观察不作为当前关闭依据，真实检查义务不因任务文件操作消失

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

### Requirement: Syntax support claims SHALL have per-language and per-host evaluation evidence

验收 MUST 分别记录合法/非法语料、ERROR/MISSING、语言版本/方言、模板位置映射、原生对照、误报/漏报、未知覆盖及白名单处置。每个声明支持的语言/宿主 MUST 有独立证据；CodeGraph 的支持清单不自动继承为 Codeguard 能力。冷/热启动、包大小、内存/并发及异常退出预算须实测后固化，不编造精度或性能数值。三个宿主的实际对话交付应分别验证，任一个通过不能替代另两个。

#### Scenario: Grammar corpus passes but native confirmation is absent
- **WHEN** 只有解析器样例成功
- **THEN** 仅记录对应解析能力，不宣称原生闭环、全语言或宿主验收完成

#### Scenario: False positives disappear through exclusions
- **WHEN** 扩大排除导致表面误报数下降
- **THEN** 评测仍保留原始分母、排除和未完成覆盖，不能据此声称准确率提高
