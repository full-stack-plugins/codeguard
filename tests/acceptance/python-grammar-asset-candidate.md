# Python WASM 固定资产候选：局部验收（2026-09-29）

对应 OpenSpec 14.1、14.2。CodeGraph 固定提交 `40f112453583a2304c4b605a3a9d6545919662bd` 的 `src/extraction/wasm/tree-sitter-python.wasm` 已复制到 `grammars/python/parser.wasm`；两者 SHA-256 均为 `a7fdc587e77bd729b9f5b783c659be23c896e305a2c374472bed7114d9e01fac`，长度均为 456131 字节。CodeGraph 引入该字节的提交 `c2503e2` 记录它对应 `tree-sitter-python 0.23.6`；上游 `v0.23.6` 标签解析为 `bffb65a8cfe4e46290331dfef0dbf0ef3679de11`。该提交的 MIT `LICENSE` 随资产保存，SHA-256 为 `d724405ce238a22c0d35769c5a36b386ad5958192efe8bbb304fb2896254575f`。固定清单同时保留 CodeGraph 自身许可证和来源身份。

先将资产清单测试的候选数量从 3 改为 4，测试因只有 3 项失败；随后加入固定字节、许可证与清单项。Rust 资产校验核对来源提交、路径、长度、SHA-256、许可证、声明 ABI 和未验收状态，并拒绝 Python 字节或许可证被替换。`WasmGrammar` 在离线测试中实测 ABI 14，能解析一个有效函数，损坏语法产生恢复错误；错误哈希和 ABI 的既有反例仍在。`codeguard grammar status --format=json` 现在列出 4 个候选、0 个已验收发行能力，Python 的缺口为 `language_qualification_and_release_pending`，且不执行解析。

本次**只增加资产候选**。Python 的语言版本/方言范围、合法与非法语料、原生 Ruff/编译器对照、worker 资源隔离、`lint python` 原生优先接线、任务复检、包内分发与宿主对话均未验收。Python WASM 不能代替原生 lint，也不赋予交付通过权威；OpenSpec 14.1、14.2 保持未完成。

验证：`cargo test --locked -p codeguard-adapters --test grammar_asset_manifest` 3/3 通过；`cargo test --locked -p codeguard-runtime --features wasm-precheck --test wasm_grammar_load` 2/2 通过；`cargo test --locked -p codeguard-cli --test grammar_status_cli` 2/2 通过。`cargo test --workspace --locked --quiet --no-fail-fast` 整体退出 0，默认 ignored 的真实原生工具测试未计作通过。全 workspace、全部 targets、启用 `codeguard-cli/wasm-precheck` 的 Clippy `-D warnings`，格式、分层检查、OpenSpec strict、清单 Draft 2020-12 schema 验证和 diff 检查均通过。本机无 Rust 1.85，MSRV 实编仍未验收。
