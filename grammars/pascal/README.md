# Pascal grammar 候选来源

CodeGuard 固定 CodeGraph 提交 `1072f82ce24db3d133258d30165cef6b74d108b2` 的 `src/extraction/wasm/tree-sitter-pascal.wasm`：716886 字节，SHA-256 `be3634fca99c19f5e1035a1a9c7d93d6ee82b35e6d5024f02be4883b71329c3e`。该提交中的字节与本目录 `parser.wasm` 完全相同。引入该 WASM 的 CodeGraph 合并提交 `77135445db0d5af66a0af57e176d73669a6b3767` 的父提交 `package.json` 将 `tree-sitter-pascal` 精确锁定为 `github:Isopod/tree-sitter-pascal#042119eca2e18a60e56317fb06ee3ba5c32cb447`。

上游该提交的 `package.json` 版本为 `0.10.2`，本目录 `LICENSE` 与上游原始字节相同，SHA-256 `2e2fcc500eacb15b76c11fd2229aebf0a5b4c8ed476f068447c6dce778a17ac9`。Rust `WasmStore` 对固定的 CodeGraph 字节实测 ABI 14，基本合法和非法 Pascal 程序及隔离 worker 通过。尚未从上游源码重新构建出与 CodeGraph 一致的 WASM；Delphi/FreePascal 方言语料、原生检查器对照、公开 lint 路由和发行包均未验收。
