# Ruby项目原生语法扫描局部验收

对应introduce-rust-codeguard-cli的native-tool-adapters Ruby项目场景及S05/S08/S09/S14，不勾选全语言或完整项目lint父任务。

## 行为

`codeguard check ruby ROOT --ruby-tool /absolute/path/to/ruby --format=json`与check all使用同一Ruby扫描器。显式入口优先，否则只选调用方绝对PATH中的ruby。固定Ruby2.6.10p210-c语法观察，不执行源码/gems。每轮最多64文件，保留unobserved_count，与整个检查共用deadline和取消标记。选定工具失败或版本不支持不转WASM；缺工具才保留候选初检。源码、工具目标或工具字节变化撤回原生位置、重检argv与当前标记。

已初始化工作区共用单文件lint的稳定任务和原工具task verify。聚合next只推荐当前选择关联的任务。原生零诊断仍不关闭任务；RuboCop、Ruby项目版本适用性、完整项目类别、可信政策和交付仍未评估。NativeCoverage把Ruby偏好与原生完成分开，不将“有工具”当作覆盖完成。

## TDD及回归

初始4项测试全部失败：缺--ruby-tool参数/缺原生扫描/缺任务与next。实现后通过，新增源码变化和65文件上限反例。默认全workspace252个结果目标合计1395通过/0失败/125忽略，日志`/private/tmp/codeguard-ruby-aggregate-workspace-default.log`。WASM6目标58通过/0失败/3忽略，覆盖Ruby单文件/项目、Swift、Zig、Rust原生优先与Hook；另行32种grammar混合项目/逐语言/编辑Hook以及语言选择的3目标26通过/0失败/1忽略。两批WASM目标分别执行，未执行全WASM workspace，不使用用户未提交的Erlang差分草稿作为验收oracle。

默认与WASM全workspace all-targets严格Clippy通过；fmt、分层、OpenSpec strict、diff检查通过。

## 实际报告和协议

默认、WASM实际CLI各捕获4种状态：缺工具、真实/usr/bin/ruby诊断、无效显式入口、修复后零诊断。WASM缺工具报告含Ruby候选；选定真实/无效工具时Ruby候选为空。修复后原生completed，finding.state仍open。8份捕获位于tests/acceptance/evidence/ruby-aggregate-2026-10-05-{default,wasm}-*.json。独立schema回归3项通过，拒绝未知协议、伪造交付许可/批准与列号。

新增check_feedback0.51与ruby_syntax_scan0.1/0.2封闭schema；旧0.50等原件不修改。实际捕获暴露旧schema的执行项/原因枚举及结构候选contains约束不适用于Ruby原生完成的空候选，均在新协议精确补齐；仍严格核验每类候选和原生位置。不以宽泛任意对象通过验证。

## 未完成边界

原生Ruby3、完整RuboCop lint/注释/依赖/CVE、安全规则、可信关闭、真实宿主及发行仍开放。CLI源码接线不证明插件已升级，也不改变公开npm0.1.4。CI增加check_all_ruby目标，远端结果独立确认。

## 远端回归待确认

前一提交87c94b3的Linux CI 37320629796在默认lib测试中失败：Go伴随工具变化用例期望go_syntax_tool_changed，实际go_syntax_version_unverified。该结果保持incomplete，未证明误通过；根因尚未确定。本批仅增加测试构建下的终止类型、spawn错误码与输出字节数诊断，以及phase/mode断言上下文，不重试、不放宽断言、不改公开报告。本机默认lib 67通过/0失败/3忽略；远端结果须独立核实，不能借用本机结果宣称Linux已修复。
