# Objective-C 与 Solidity 依赖 grammar：局部验收

CodeGraph 固定提交 `1072f82ce24db3d133258d30165cef6b74d108b2` 的锁文件选用 `tree-sitter-wasms@0.1.13`。本仓固定该 npm 包的 SHA-512 完整性、随附 Unlicense、两份原始 WASM 的 SHA-256 和字节长度，并保留 Objective-C 与 Solidity grammar 的 MIT 许可证。来源包构建时锁定 `tree-sitter-objc@2.1.0` 及 Solidity 提交 `b239a95f94cfcc6e7b3e961bc73a28d55e214f02`。

这两份原始 WASM 使用旧 `dylink` 自定义段，当前 Rust Tree-sitter `WasmStore` 要求 `dylink.0`。仓内保留 `source.wasm`，只把四个内存/函数表尺寸参数封入新版动态链接元数据，输出为 `parser.wasm`。Rust 适配器严格校验输入和输出 SHA-256，拒绝额外依赖、未知语言和漂移字节；没有重写 grammar 的代码段。

本地实测：

- `cargo test -p codeguard-adapters --test grammar_asset_manifest`：5 通过，0 失败；包含来源、许可证、适配确定性和篡改拒绝。
- `cargo test --features wasm-precheck -p codeguard-runtime --test wasm_grammar_load dependency_grammars_load_offline_and_parse_basic_fixtures`：1 通过，0 失败；两份资产均由 Rust 离线加载，合法与未闭合样例给出不同解析结果。
- `cargo test --features wasm-precheck -p codeguard-cli --test syntax_worker_candidate dependency_grammars_run_in_worker_without_claiming_clean`：1 通过，0 失败；隔离 worker 的合法样例仍返回 `incomplete` 和 `grammar_qualified=false`。
- `cargo test --features wasm-precheck -p codeguard-cli --test grammar_status_cli`：2 通过，0 失败；32 种来源、7 份候选、0 项已发行。

尚未验收 Objective-C/Solidity 的语言版本、方言、真实项目语料、原生 lint 对照、完整资源预算、公开 `lint` 路由及发行包安装。两份资产只是候选观察；原生 lint 缺失时仍须保留环境阻塞，不能把零恢复节点判为 clean 或关闭任务。
