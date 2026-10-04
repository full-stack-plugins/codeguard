# S14.2 Rust WASM 加载器完成核验

实现与验收基线：`8ca3bb1c2381daefd9c8770fe3b5721c5a0b20e6`。只关闭加载器任务14.2，不关闭资产治理14.1、隔离14.3、语言覆盖14.4、精度14.17、发行14.18或全量语言验收14.19。

| 14.2验收条件 | 当前证据 | 结论与范围 |
|---|---|---|
| 固定资产离线加载 | `crates/codeguard-runtime/src/wasm_grammar.rs`在构造Engine/WasmStore前核验WASM头、尺寸、固定SHA-256，加载后比对真实ABI并设置Parser | 加载器接受调用方固定清单中的候选资产；不授予grammar质量或发行资格 |
| 不依赖Node解析器、CodeGraph或用户安装tree-sitter-cli | runtime依赖固定`tree-sitter=0.25.10`的wasm特性，直接调用Rust `WasmStore::load_language`；测试使用随仓`include_bytes!`资产 | 运行时没有Node/CodeGraph/生成器调用；npm启动器与开发期grammar生成工具不是解析后端 |
| 有效资产与基础解析 | `wasm_grammar_load`的11个正向测试覆盖全部32份固定grammar，使用真实导出名与ABI | 12项加载测试实际通过；其中第12项为负例，不将加载通过当成完整语言准确性验收 |
| 损坏字节、错散列、错ABI拒绝 | `invalid_bytes_hash_and_abi_are_not_accepted`拒绝坏名字、坏字节、合法头的空模块、错散列、Java错误ABI；原始不兼容Zig/Dart模块另有反例 | 拒绝不是项目源码违规，保持候选加载边界 |
| 实际最低Rust版本 | [CI 37243591502的MSRV job](https://github.com/full-stack-plugins/codeguard/actions/runs/37243591502/job/111556990745)在上述提交使用Rust1.85.0，default及wasm-precheck workspace/all-targets检查均成功 | 是真实Rust1.85检查；不是本机新版编译器代替MSRV，也不是完整CI gate已成功 |

本机执行：

```bash
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked --offline -p codeguard-runtime \
  --features wasm-precheck --test wasm_empty_block_scan \
  --test wasm_recovery_scan --test wasm_grammar_load
```

三个目标共21通过、0失败；加载目标12通过，包含全部32份资产和负例。合法Rust空块正对照随后单独在新扫描目标4通过。默认/WASM workspace/all-targets严格Clippy、OpenSpec strict和分层检查通过。相关CI的MSRV已成功，gate仍在运行，不据此声称全部CI通过。

仍未完成：32种grammar的独立原生精度/版本/方言/性能验收，已知Erlang10个漏检、VB.NET1个误报和Python2个漏检，跨平台内存/取消边界，正式发布资格及宿主闭环。已验收发行grammar仍为0。资产来源/许可/方言治理由14.1继续处理；本任务关闭不会把未批准候选变成发行资产。
