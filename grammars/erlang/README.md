# Erlang grammar 候选来源

CodeGuard 固定 CodeGraph 提交 `1072f82ce24db3d133258d30165cef6b74d108b2` 的 `src/extraction/wasm/tree-sitter-erlang.wasm`：421639 字节，SHA-256 `dbab33f03e07b89f4385fcdd48d87d86ba35c82a0a426788d55b8c25410bc491`。该提交中的字节与 `parser.wasm` 完全相同。CodeGraph 的 [Erlang 接入说明](https://github.com/colbymchenry/codegraph/pull/1165) 标明来源是 `WhatsApp/tree-sitter-erlang` 标签 `0.19`，并提醒不能以同名 npm 包替代该来源。

标签 `0.19` 指向提交 `836aa2b6c3af2c7cef3f84049b0ed6d44485a870`；`LICENSE` 从该提交复制，SHA-256 `c71d239df91726fc519c6eb72d318ec65820627232b2f796219e87dcf35d0ab4`。Rust `WasmStore` 对固定的 CodeGraph 字节实测 ABI 14，基本合法和非法函数样例及隔离 worker 通过。此处未重新构建并比对上游标签生成的 WASM；语言版本、方言语料、原生工具对照、公开 lint 路由和发行包均未验收。
