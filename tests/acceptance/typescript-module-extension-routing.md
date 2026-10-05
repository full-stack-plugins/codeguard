# TypeScript 模块后缀项目发现与原生优先路由

对应 introduce-rust-codeguard-cli 的 14.4、14.6、14.10、14.19；父任务仍开放。

## 依据和缺陷

[TypeScript 4.7 官方说明](https://www.typescriptlang.org/docs/handbook/release-notes/typescript-4-7) 明确 `.mts/.cts` 源码及 `.d.mts/.d.cts` 声明后缀。现有单文件 TypeScript 入口、ESLint 输入与证据验证已经支持这些后缀，但共享语言注册表和聚合 grammar 路由只识别 `.ts`。只有模块后缀的项目因此报告零源码和零初检。

## RED / GREEN 证据

修正前实际执行三个失败入口：

- grammar_route：module.mts 返回零路由，期望一个。
- 默认构建 detect_cli：项目没有 TypeScript 语言。
- 实际 check all：4 个源码/声明文件被报告为 source_file_count=0。

最小修正仅追加注册表 `.mts/.cts`，并将两种扩展名路由到固定 TypeScript grammar；不扩展 TSX、未知 `.mtsx/.ctsx` 不作猜测。实际项目测试修正后 1 passed，2.86 秒：两份非法源码产生有界恢复，两份合法声明零恢复；所有四份候选已观察但 grammar_qualified=false、delivery_decision=incomplete。

已初始化工作区的重复 check typescript 与编辑 Hook 回归 1 passed，15.25 秒：两份非法文件保持相同任务 ID，合法声明不创建确认任务；只编辑 module.mts 时仅观察该文件并复用原任务。测试准备中 init 退出码按既有未完成门禁为3，而不是0；check 使用既有分隔式 --timeout 60s。首次测试准备的错误断言/参数失败未计为产品缺陷。

补充受控原生优先协议夹具：同模块两种后缀有完整 ESLint 发现时不重复 WASM，另一个构建根无上下文的同后缀文件继续独立初检。该夹具验证调度，不充当真实 TypeScript 编译器或原生规则精度 oracle。

受影响七目标共85 passed / 0 failed / 0 ignored，包括全32份聚合及编辑Hook回归。首轮已有Ruff测试继承宿主PATH，自动发现Zig导致覆盖计数2而非1；固定该夹具PATH后通过，显式Ruff仍执行且Zig文件实际走WASM。默认/WASM严格Clippy、OpenSpec strict及diff检查通过。不改变 grammar 字节、原生检查能力、规则批准、候选资格、公开 npm 或插件制品锁。
