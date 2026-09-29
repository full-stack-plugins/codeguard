# CFML grammar 候选来源

固定 CodeGraph `1072f82ce24db3d133258d30165cef6b74d108b2` 的 `tree-sitter-cfml.wasm`，SHA-256 `17aeb294c58dd47045b470fc082ccceac3100c8e548b13764d4f3987a062322f`。CodeGraph [CFML 接入记录](https://github.com/colbymchenry/codegraph/pull/1153) 声明三份 WASM 从 `cfmleditor/tree-sitter-cfml` 重建并逐字节一致，约为 `v0.26.29`；本清单固定该标签提交 `5279bde9b31d54efa68efb77a4b18ee9508518e4`。`LICENSE` 来自该提交，SHA-256 `7e17ba5e26b0438ca04ebf2d9e7931da3080509da6625e117f6feb943ecf767a`。CodeGuard 自身已核对 CodeGraph 固定提交的制品字节，尚未独立重复上游构建。

Rust 离线加载实测 ABI 15，基础正反例及隔离 worker 通过。CFML 文件还涉及 tag、bare-script、`<cfscript>` 和 `<cfquery>` 的语境切换；仅加载此单一 grammar 不等于完成这些语境或原生检查验收。
