# Dart grammar 重建候选局部验收

日期：2026-09-29。对应 OpenSpec 14.1、14.2、14.4、14.19；整体任务未完成。

固定 CodeGraph `1072f82ce24db3d133258d30165cef6b74d108b2` 原 Dart WASM 的 SHA-256 为 `7f5364e4256cf7e55efd01dd52421ef2663caa8061b82659b7e4bf61064545ec`。旧 `dylink` 元数据定界转换后，Rust 仍因 `tree_sitter_dart_external_scanner_create` 未解析而拒绝实例化。该 scanner 负责字符串模板和注释；空函数不能代表语法实现。

重建取自 [UserNobody14/tree-sitter-dart 固定提交](https://github.com/UserNobody14/tree-sitter-dart/tree/d4d8f3e337d8be23be27ffc35a0aef972343cd54) 的 C 解析器、真实 `scanner.c`、三份头文件与 MIT 许可证。逐文件与上游原始字节核对通过；摘要详见 [Dart 资产说明](../../grammars/dart/README.md)。Zig 0.16.0 `zig cc -target wasm32-wasi` 将解析器和 scanner 一并编译。两次独立目录构建的 `source.wasm` 均与提交资产逐字节一致；受控 Rust 导入适配仅改 import section 中一个同签名 WASI 名称，适配后字节也能重复生成。

RED：旧 CodeGraph WASM 报 `failed to parse dylink section`，只改元数据仍报外部 scanner 导入无效；新重建的未适配 WASM 报 `invalid import '__main_argc_argv'`。GREEN：固定适配资产由 Rust 离线加载，实测 ABI 15；普通函数、字符串插值、文档注释、块注释正例无语法恢复，未闭合函数、字符串和注释反例产生恢复错误。固定上游提交的 15 份 corpus 文件共 150 例（预期正常 146、预期错误 4），Rust 解析错误分类全部一致。隔离 worker 对 Dart 正例返回 `incomplete`、`grammar_qualified=false`。更改重建源字节被适配器拒绝。`grammar status` 区分 CodeGraph 原资产、重建源与候选 WASM，报告 32 种来源 / 23 份候选 / 0 项已发行，退出码 3。

这只是候选资产验收。尚需 Dart 版本与方言语料、原生 `dart analyze` 对照、误报和漏报统计、公开原生优先路由、工作台与宿主对话反馈、资源预算、发行包离线实装和多平台验证。不能把可加载的语法树或初检零恢复当作 lint 通过。

完整回归初轮在已有 4 秒 worker 超时断言上失败；单独复测通过。排查发现父进程为单文件候选也核验全部资产，与先前仅优化子进程的设计不一致。父进程改为只核验所选资产后，`syntax_worker_candidate` 13/13 并行用例通过；这修复了可避免的全量散列开销，但单次运行时间不能替代正式冷暖启动和资源评测。

第二轮全仓 `cargo test --locked --workspace --all-features` 继续在两个既有 Maven 探针用例上失败：并行负载下，固定 2 秒的版本探针先返回 `version_timed_out`，因此没有到达测试预期的运行时/工具包字节变更判定。单独执行 `cargo test --locked -p codeguard-cli --test maven_probe_contract`，9 项有效用例全部通过，3 项因缺固定 Maven/JDK 环境而忽略。全仓并行门禁尚无成功证据，不据此宣称完成；CI 增加 Dart corpus 与 WASM 加载测试，待 Linux 运行结果确认。

串行全仓 `cargo test --locked --workspace --all-features -- --test-threads=1` 通过：183 组、1146 项通过、0 项失败、106 项忽略。该结果证明当前源码的串行回归；并行全仓结果仍保留上述超时限制。`cargo fmt --all -- --check`、全工作区全特性 Clippy `-D warnings`、OpenSpec 严格校验及 grammar manifest Draft 2020-12 Schema 校验通过。
