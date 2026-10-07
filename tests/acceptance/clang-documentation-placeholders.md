# C-family 明确占位说明观察

对应 introduce-rust-codeguard-cli 的详细文档规范要求。新增独立适配器 parse_clang_documentation_placeholders 和封闭协议 clang-documentation-placeholder-v0.1；旧结构协议 nonempty 的含义保持不变。

仅整个受支持说明经空白与句末标点归一化后等于 TODO、TBD、FIXME、待补充或待完善时，报告 codeguard.documentation.placeholder_description。用途、命名参数及适用返回分别定位；输出不含注释原文。含有标记的正常解释、void 返回和结构解析器不支持的上下文不推断违规。此规则属于 Codeguard 自有策略，不冒充 Clang 原生警告，不证明剩余文本的语义准确性。

测试先因缺失能力失败。实现后合成正反例与真实 Clang C11 AST 测试均通过：2 项通过、0 忽略。实际原生测试首次把函数行号误写为5而失败，纠正为源码第4行后通过。实际输出通过 Draft202012 schema 校验；3 个伪造 qualification、coverage 和 authority 的变异被拒绝。完整命令如下：

```sh
CARGO_PROFILE_TEST_DEBUG=0 CODEGUARD_CLANG_BIN=/usr/bin/clang \
  cargo test --offline --locked -p codeguard-adapters \
  --test clang_documentation_placeholder_contract -- --include-ignored
```

可用 CODEGUARD_PLACEHOLDER_EVIDENCE 指定绝对路径保存实际观察供 schema 校验。该输出是局部观察，不是绑定发布制品的生产资格收据。

未完成：公开 comments 命令、稳定任务与修复指引、原任务 verify、尝试输入规则身份、历史消费兼容、白名单与可信关闭、跨语言/平台独立精度验收。新文件不会因存在或测试通过而使语言生产义务升级，相关父任务继续未勾选。

## 公开单文件反馈接入

公开 comments c/cpp 已以反馈0.10暴露 documentation_placeholders；警告、结构和占位观察共用同次冻结stdin原生扫描。source变化、工具不可用、取消/截止仍撤回占位观察；文本模式输出组件与位置。shared observe 的历史 check/hook 消费和结构/警告任务存储继续使用原协议，尚未为占位签发任务权限。新增固定探针档案共用现有文档警告参数，不增加编译器调用。

默认与WASM公开占位测试分别1项通过；已有公开结构1项和工作台16项在两个构建模式均通过。结构反馈的24份实际新报告通过封闭schema，12个伪造覆盖/资格/结构/占位权威等反例被拒绝，489份历史schema逐字节不变。新schema首次因动作数组上限和内嵌分支$defs引用问题失败，独立版本内修正后通过；原schema未改。

未完成范围更新：稳定占位任务、原任务复检、尝试摘要策略身份、可信关闭、共享check/hook自动触发与对话投影仍待接入。公开反馈明确placeholder_task_workflow_status=not_integrated，原结构任务清空不能被解释为占位文档合格。

## 消费关联校验

新增 valid_clang_documentation_placeholders：封闭形状、固定策略权威和版本、组件位置唯一性，以及对同次已关联原生结构/可选冻结源码的匹配。未知参数、void返回、错位源码、重复位置组件、额外字段与伪造资格均拒绝。原生探针在暴露位置前调用该校验，失败即撤回占位观察。

反例测试先因缺少能力失败，实现后1项通过；公开真实Clang占位回归1项通过。该验证不从本地可编辑文本证明占位事实为真，未来稳定任务消费仍须首次原始报告、工具/源码身份和消费收据。稳定任务、原任务复检和共享自动触发仍未完成。

## 独立工作台包导入

work sync 已接入 clang_documentation_placeholder_workbench_observation 0.1，校验原生/结构/占位封闭协议及当前工具、源码、工作区和运行身份。独立文件任务按路径、语言、标准和策略稳定计算，行号漂移不新建；局部清空不关闭历史fact。未知参数伪造包不产生消费收据或新任务。

真实Clang公开扫描输出构造局部导入包的CLI集成测试1项通过，验证3次合法消费、稳定身份、open保留、伪造组件拒绝及专用复检缺口。首轮测试因运行ID不符合已有四段数字契约而失败，修正测试输入后通过。此为本地报告导入验收，不是独立精度或发布资格。

当前next/task verify对该独立检查器明确返回clang_placeholder_task_workflow_not_integrated，不执行其他检查器或写复检事件。公开comments的自动持久化、完整任务指引和专用原工具复检仍待接线，父任务继续未完成。

## 公开自动持久化

公开comments c/cpp在已初始化绑定工作区自动保存并消费独立占位包，反馈0.11返回placeholder_workbench状态和最多一个稳定任务ID；未初始化反馈继续0.10且不建目录。不可用观察不保存有效占位包、不返回任务ID。局部消失仅清空本轮ID，历史fact仍open；专用next/verify保持具体未接入原因。

默认和WASM各1项真实测试均覆盖C11/C++17的未初始化、修复/缺工具、初始化、重复稳定任务、消费收据、清空保留open及缺工具持久化未完成。7份实际公开反馈通过0.10/0.11封闭schema校验；既有结构公开回归默认/WASM各1项通过。扩展C++测试时首次误用C11标准，修正为C++17后通过。适配器和协议来源纳入c/cpp standalone documentation的部分实现引用，未提升资格。

当前未完成：专用任务当前定位/历史撤权、受控尝试与原工具task verify、可信关闭和复发重开、共享check/hook自动触发、白名单及独立跨平台精度验收。之前各节的缺口描述属于相应历史检查点，以本节为当前接入状态。

## 收据绑定的任务诊断视图

next的独立0.34诊断视图核验首次不可变事实、报告摘要、工作区/路径/语言/规则身份和消费收据，选择最新已消费同文件观察并保持原工具及标准。当前占位组件可见；源码变化撤回定位，收据被篡改不返回可用brief。视图保留原comments复扫argv、规则依据、约束和具体集成缺口，allowed_paths为空且attempt_history_status=not_integrated，不能以假历史或已有任务存在来签发修复权限。

默认/WASM各1项真实C11/C++17公开测试覆盖当前定位、源码变化撤权和收据篡改拒绝；独立导入回归及专用verify不执行其他检查器仍通过。2份实际只读视图通过封闭schema，4个伪造允许范围、历史、权威、可执行状态的变异被拒绝。该集成缺口是工程待办，不是已批准例外或用户必须批准的误报决策。

下一步仍是受控尝试/原任务复检接线与验收；可信关闭、共享check/hook投影及跨平台独立精度仍未完成。

## 原任务复检身份前置检查

专用preflight已核验首次独立占位包及摘要、精确消费收据、open不可变任务身份/路径/工作区/规则/原源码与报告摘要，并要求原工具路径及原观测工具字节。替换路径在租约/执行前拒绝；其他检查器参数返回错参2，不执行检查器或持久化复检事件。健康身份仍返回workflow-not-integrated，实际执行/结果消费/受控尝试未接入，不把身份检查当复检成功。

真实Clang导入/任务测试通过原工具未接入状态、替换工具拒绝、外来Ruff参数拒绝及无native_scan/event；既有稳定身份、源码修复局部消失不关闭与伪造包拒绝保持。父任务和生产资格未提升。

## 原工具执行与局部事件（当前增量）

`task verify` 已接入首次报告及原工具身份绑定后的实际有界复检，使用既有租约、锁、工作同步和追加事件事务。当前C11/C++17真实公开测试覆盖仍存在与修复后局部消失；后者仅为 `candidate_absent_unverified_policy`，历史事实保持 open。伪造待消费报告使同步返回 `backlog_sync_incomplete`，复检报告保留局部结果但不声称事件已持久化，不删除或绕过非法历史。

内部协议为 [placeholder task recheck 0.1](../../schemas/clang-documentation-placeholder-task-recheck-v0.1.schema.json)，公开协议为 [task verification 0.38](../../schemas/task-verification-preview-v0.38.schema.json)。真实报文及反例验证入口是 [协议验证器](../c_family_placeholder_task_recheck_schema.py)；设置 `CODEGUARD_PLACEHOLDER_RECHECK_EVIDENCE` 为绝对前缀，公开测试输出 `.c.json` 和 `.cpp.json`，避免两语言证据相互覆盖。协议通过不能证明内容语义准确或可信修复完成。

受控尝试与无进展预算尚未接入；执行失败、超时及AST不可用返回 incomplete，但完整失败观察的持久化仍须补齐。可信关闭/复发、白名单、共享check/hook、独立精度及平台发行验收继续未完成。此前未执行的描述是历史检查点，不是当前状态。

本增量验证：默认/WASM模式公开两语言占位任务测试各1通过，导入/非法历史阻塞测试各1通过，计划回归各6通过；实际C与C++四份0.38报文通过封闭schema，24项资格/字段/失稳篡改被拒绝。WASM全工作区all-targets严格Clippy、分层检查及OpenSpec严格验证通过。以上仅为本增量开发验证，未授予生产资格。

## 受控尝试与同输入无进展（当前增量）

占位任务现使用共享租约和追加事件账本，`repair-source`开始后指引为waiting，ready-to-verify后要求原工具复检。输入摘要绑定当前源码、首次语言/标准/策略、当前工具字节、占位解析/验证及底层结构实现、专用复检实现。历史结构复检不充作占位复检。

同一输入两次仍存在撤回allowed_paths与占位定位，并拒绝第三次尝试。重复扫描与删除任务Markdown不重置预算；缺早先复检证据转具体决策，篡改报告撤回整个指引。只允许修改当前文件文档；API/行为、检查配置与白名单不能由任务自行改变。新增next 0.35及task show 0.8，历史只读0.34协议保持不变。

真实验收入口：[CLI受控尝试测试](../../crates/codeguard-cli/tests/c_family_placeholder_attempt.rs)、[协议与预算验证器](../c_family_placeholder_attempt_schema.py)。完整失败观察持久化、跨输入语义无进展、详细内容语义准确性、可信关闭与复发、check/hook、白名单和独立生产精度仍未完成。

本受控尝试增量验证：默认及WASM真实C11/C++17各1测试通过；公开反馈、非法历史导入、原结构闭环各1回归通过，计划各6通过。12份实际初始/耗尽指引、task show及原复检报文通过协议，10项额外字段、错规则、预算耗尽仍授予权限的反例被拒绝；WASM全工作区all-targets严格Clippy、分层和OpenSpec严格验证通过。以上均为开发增量验证，生产资格不变。
