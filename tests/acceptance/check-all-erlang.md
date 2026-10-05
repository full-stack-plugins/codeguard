# check all 的 Erlang 原生 forms 接线验收

日期：2026-10-04。关联既有 OpenSpec 8.134、14.5–14.9、14.17、14.19；父任务仍未完成。

## 当前行为与范围

`check all` 新增 `erlang.lint` 任务图节点，共用请求 deadline/jobs。显式 `--erl-tool` 优先，否则固定 PATH 绝对目录的首个可执行 erl；不因所选工具版本或执行失败改选其它同名工具。复用 OTP 28 原生 scanner/parser，不展开宏、不调用 parse_transform、不执行项目源码。最多 64 个普通 UTF-8 文件，每文件 1 MiB；超范围、工具缺失、宏/预处理、时间和诊断预算均可见。

逐文件反馈保留当前源码摘要、原生位置、下一步和绝对源码路径的原工具复检 argv；从项目外调用和重放均实际验收。源码或工具变化撤回受影响的当前诊断；项目范围变化只撤回 scope_stable/local_forms_complete，保留仍与当前字节匹配的单文件发现。完整且匹配的本轮 forms 观察才跳过重复 WASM，宏/部分结果不跳过。SARIF 保留原生 finding，同时声明未完成；Java 选择和错误参数不启动 Erlang，取消保持 130 并回收子孙进程。

聚合协议升级为 check_feedback 0.36.0、check_aborted 0.13.0；内嵌 erlang_forms_scan 0.1.0，旧 0.35/0.12 schema 原件保存。结构不能表达本地可信权威、项目通过、全量覆盖或虚构任务 ID。

## RED 与验收证据

最初新增入口回归 0 passed/6 failed/1 ignored，暴露缺少原生结果、参数接线、SARIF 发现和过期指引处理；日志 `/tmp/codeguard-check-erlang-red.log`。WASM 去重初次失败于错误复用 Go 后缀过滤，修正为 Erlang/HRL 的同字节判断。项目范围变化保留有效文件发现的新增反例先 0 passed/1 failed，日志 `/tmp/codeguard-check-erlang-scope-red.log`，随后通过。跨工作目录复检反例另先 RED，暴露相对路径错误，改为绝对源码路径后原工具重放通过；日志 `/tmp/codeguard-check-erlang-cwd-{red,green}.log`。

真实 OTP 28 双文件回放：合法文件 completed、缺句点文件 diagnostics_observed，保留 2:8 原生位置，两文件都跳过重复 WASM。显式真实工具用例 1 passed/0 failed/0 ignored；日志 `/tmp/codeguard-check-erlang-real.log`。这不覆盖完整项目 lint、预处理或版本精度。

局部特性回归、32 份路由、默认全工作区终态和日志身份见本变更 verification 的同名小节。忽略项不计通过；不同测试阶段有重叠，不相加声称独立测试数。

实际报告位于 `/tmp/codeguard-check-erlang-reports/{native,missing,bad}.json`；原生缺句点、缺工具和显式无效工具分别通过聚合与独立 scan schema。中英文技术文档的完整内嵌示例来自实际 OTP 输出，只证明结构/同轮观察，不证明供应链权威。旧 aggregate schema 必须与本批开始前 Git 版本逐字节一致。

## 未完成与发行边界

原生发现的持久任务、完整 Erlang lint/预处理/注释/编译/测试、可信关闭、宿主自动触发、全语言精度和性能均未完成；task_id 明确为空。已有 WASM 来源的 Erlang 确认任务仍由独立 task verify/repair_ready 处理。37 例 Erlang grammar 扩展草稿仍因 10 项差异失败，未纳入本批提交或计作通过。公开 npm 0.1.4、插件锁和发行资产未改变。

最终默认全量207组、1148 passed/0 failed/108 ignored；最终Erlang特性目标11 passed/0 failed/1 ignored，显式真实OTP1 passed；全目标特性Clippy、186份schema/实际报告/双语例子、12类矛盾反例、fmt/分层/OpenSpec strict/链接通过。阶段范围与失败保留见 verification，不把部分或忽略项计作完成。
