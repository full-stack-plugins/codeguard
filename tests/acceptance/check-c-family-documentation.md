# C/C++统一check文档局部验收

规格事实源：introduce-rust-codeguard-cli/native-tool-adapters 的统一C-family文档调度要求；对应2.3/6.1/6.2/9.9/15.3/15.6。既有66完成/288未完成、228核心blocked和0/32 grammar生产资格不变。

## 当前行为

`check all/c/cpp`新增显式`--clang-tool ABS`、`--c-standard c11`、`--cpp-standard c++17`，按发现的所选语言调度原文档警告与AST观察。独立comments与统一入口复用同一观察器；共享任务图deadline、取消、jobs及Clang资源锁，不重置每文件整轮预算。每语言最多64文件，累计完整反馈预算16MiB，超额尾部保留未观察数。完整项目配置/覆盖仍未知。

全项目输入复核后才串联已有工作台原警告与结构稳定任务；重复扫描保留任务身份。未初始化项目不隐式创建`.codeguard/`。上下文缺失返回context_required；缺工具保留环境反馈，不制造源码违规。输入/范围/工具变化、共享期限或取消撤回当前定位及next；未完成不能作为修复关闭或门禁许可。报告0.72、中止0.22、语言扫描0.1及集合0.1为新封闭协议；503份历史schema保持原字节。SARIF区分原生诊断与CodeGuard自有结构规则，始终executionSuccessful=false。

## 测试与证据

- 默认/WASM各37项公开目标：原comments9、公开结构1、原警告工作台16、结构工作台1、新check5、help5。5项新check覆盖真实已安装Apple Clang21双语言/单语言、稳定任务、未初始化、缺上下文/缺工具、短deadline、错参、受控原编译器包装器引起的跨文件变化、受控SIGINT/子孙清理、65文件的64上限和实际SARIF输出。两项显式真实Clang测试用include-ignored实际执行；受控程序不能计作独立原生资格。
- 两构建各3项SARIF单元回归区分原生/自有来源并丢弃旧输入；各1项中止构造器检查仅为受控协议证据，不声称已触发真实内部故障。
- 当前协议证据见[evidence默认](evidence/check-c-family-documentation-default.json)、[evidence WASM](evidence/check-c-family-documentation-wasm.json)和[schema回归](evidence/check-c-family-documentation-schema.json)。记录测试源码及真实运行二进制摘要，真实Clang身份在子报告中保留；校验20份check、28份语言扫描、150份子报告、2份受控中止，11项伪造反例拒绝。
- 源码使用offline/locked构建。default/WASM全工作区all-targets严格Clippy实际通过；layering、既有OpenSpec严格验证、双语架构命名及diff检查通过。相邻默认入口22项partial、4项选择（含显式真实Ruff）、6项SARIF及4项生产计划通过；249份来源摘要与1312处任务引用实际核对。未执行不计通过。

## 剩余边界

仍为Apple Clang21独立C11/C++17档案。项目编译选项/头文件/预处理、全部对象、错误/行为契约准确性、可信关闭/复发、跨输入语义无进展、自动Hook、独立precision/recall、完整平台及发行未验收。原生警告与自有结构缺失分层，非空描述不代表准确说明，零警告不代表详细文档合规。父任务不勾选，不宣称四核心生产就绪。

远端CI37546714851（cb0b42d）已确认终止失败，原因是固定插件审计提交dec5f9d未在插件远端可达；不删除该检查或改写来源身份。
