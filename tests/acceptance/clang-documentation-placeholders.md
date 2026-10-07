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
