# CFQuery DISTINCT 投影候选及嵌入任务接线

日期：2026-10-06。对应OpenSpec12.11、14.4/14.6/14.7/14.9/14.17/14.19及syntax-precheck；父任务仍未完成。

## 反例与规则

已有固定PostgreSQL18.6原生PREPARE反证接受SELECT FROM users、拒绝SELECT DISTINCT FROM users（42601），而固定WASM均零恢复。原生证据与历史grammar报告保留不改。worker缺少结构候选的公开反例先RED；新增Rust runtime的有界直接AST关键词事实，适配层提供版本化规则摘要，CLI解释为独立结构候选。不是搜索整份源码或伪造ERROR/MISSING。

字符串、引号标识符、标识符表达式、CFML hash插值打断序列；仅AST comment跳过。case-insensitive关键词及SQL注释可匹配；128记录/200000直接节点预算截断明确可见。父进程核对固定规则及源码锚点，不接受锚点中的其它表达式、未闭合注释或伪造rule摘要。候选要求原生确认，不能据此删源码、批准白名单或关闭任务。

## 公共入口与身份

新worker1.3、probe0.4、check0.53、Hook0.23/0.13、保存候选0.11保持旧协议文件不改。嵌入片段source_sha256绑定完整原文件，fragment_source_sha256保留实际worker字节；恢复和结构坐标映射回原文件。此前raw片段SHA被当成完整文件SHA，无法形成正确工作台任务，本轮分开核验。重复check/edit同一路径语种复用稳定CG-B任务，修复后任务仍open。Claude格式列举实际结构规则，不再把所有语言的结构候选称为Python。

next说明先明确datasource、方言/版本和模板/schema上下文；当前CFQuery原生task verify未接入，给出具体能力决策，不建议安装其它语言工具、不自动连接项目数据库。

## 验证状态

最终CFQuery集成4通过、0失败，覆盖独立结构规则、合法/字符串/注释/插值反例、完整文件/片段身份与稳定任务以及128记录截断。全32候选项目/编辑路由回归14通过，另1条旧提示断言失败后改为要求原生方言事实，并单独复检；具体终态见下面追加记录。此前五目标共享回归32通过、0失败、2条件忽略为片段摘要修正前的局部基线，不当作最终全workspace证明。显式固定PostgreSQL隔离夹具3通过，无网络、禁止拉取、PREPARE不执行待检查询并清理本轮容器。

实际命令捕获包括普通空投影/ DISTINCT probe、两次Hook、check all、持久确认报告、next及Claude格式重放，六类报告通过schema；封闭schema负例、双摘要、完整源码字节锚点、稳定任务及对话脱敏回归4通过。证据：[实际输出](evidence/cfquery-structure-2026-10-06.json)，记录实际二进制SHA-256。复现：WASM构建后运行`python3 tests/capture_cfquery_structure_feedback.py`及`python3 tests/cfquery_structure_feedback_schema.py`；Python仅为开发期JSON Schema验收工具，产品扫描与worker均为Rust。

当前manifest提示纠正为PostgreSQL18.6接受普通空投影、拒绝DISTINCT变体；只修改当前已知限制文字，历史manifest、精度矩阵、WASM字节和许可证不改。实际宿主会话、完整项目、全部语言资格与发行仍没有验收。

## 剩余范围

这两条原生样例不是独立holdout。固定grammar原始FN及全部历史矩阵不改；补充规则不等于重建grammar，不声明全局低误报率。32份候选资格仍0；数据库方言自动绑定、全部动态模板、schema/注入安全、项目SQL原生adapter、可信关闭、真实宿主和发行仍未完成。用户Erlang草稿保持原样，不执行、不提交。

终态补录：旧提示断言修正后单目标1通过/0失败，另14项路由测试已在修正后的生产源码上通过；没有将失败批次声称全通过，也未累加重复运行。OpenSpec严格验证、crate分层与diff检查通过。

最终default/WASM两构建workspace/all-targets严格Clippy均通过（包含当前manifest提示修正）。用户Erlang草稿摘要保持不变，未执行、未纳入提交。
