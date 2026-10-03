# Erlang 源文件终止符修复：待重建与验收

日期：2026-10-04。对应既有 OpenSpec 14.4、14.17、14.19；状态 **RED，未完成**。

## 根因及可审查补丁

固定上游 `WhatsApp/tree-sitter-erlang@836aa2b6c3af2c7cef3f84049b0ed6d44485a870` 的 `fun_decl` 使用可选函数子句分隔符，`source_file` 还允许编辑器表达式片段。原 CodeGraph WASM 仍在使用，未替换任何已发布制品。

[源码模式补丁](../../scripts/grammars/erlang-source-forms.patch)准备按完整源文件 forms 解析，把同一函数的内部子句用分号连接，并要求最终句点。该补丁尚未通过生成器冲突检查、Rust 加载或精度复验；不能把补丁存在当作修复完成。没有通过文本末尾检查伪造 ERROR/MISSING 或关闭任务。

## 独立原生标签与 RED

新增 [24 份固定语料](../fixtures/erlang_source_forms.json)：13 合法、11 非法。本机既有 OTP 28 `erlc` 已逐例独立确认标签；每例单独输出目录，输入字节未修改。加上原有 13 例，共 37 例、21 合法、16 非法。

普通扩展回归在原 WASM 上失败，报告 10 项差异：原缺句点反例，加 EOF、尾注释、Unicode/CRLF、浮点数、点字符、字符串内句点的缺终止符变体、最后分号、多子句末尾分号，以及中间缺句点。既有函数、完整多子句、合法注释/字符串/浮点数/字符、宏及条件编译样例保留，不能为修复反例而删去合法样例。

回归目标要求全部 37 例无差异、无未解析；当前未达成。显式原生测试还会检查 `lint erlang --erl-tool` 同字节反馈，宏/条件编译保持 `incomplete`。源码模式 grammar 仍不承担预处理、类型或语义判定。

## 重建前置与完成条件

本机已有 Zig 0.16.0，但未找到 tree-sitter-cli。已请求允许在临时目录下载并运行官方 0.27.0 macOS arm64 工具；未收到授权前不下载、安装或运行该工具，不改全局 PATH。官方压缩文件摘要已由 GitHub release API 核对为 `70f7573b2b2e5371a5b58cc5227d2ad981fd5374596b9874e770af486060774e`。

必须在授权后完成：固定原始源码与补丁、生成器冲突检查、真实 external scanner 编译、ABI/导入兼容核验、固定原始/派生 WASM、两次独立构建字节比较、37 例原生差分、恢复位置与资源边界回归、32 份整体路由和安装包验收。完整语言/版本精度、项目原生优先、实际宿主和正式发行仍是父任务要求；不能以这一组语料勾选父任务。

## 证据身份

| 文件 | SHA-256 |
| --- | --- |
| `/tmp/codeguard-erlang-grammar-fix-red.log` | `068ea9652b51a8b19cd5e8f89cd07628699cae29e9dcfcbe83a9feb2693eabe2` |
| `/tmp/codeguard-erlang-grammar-expanded-red.log` | `8663ce82d68b3492d25e9a5a1bebdea1c314dd9c9d5ac8886c21c490a39ec636` |
| `scripts/grammars/erlang-source-forms.patch` | `c1b64d91d0a13b8afc31ca8385d6d4fbf410e8286acdfc909e664e500eb90b07` |
| `tests/fixtures/erlang_source_forms.json` | `6e04aa01a3273e073297ac376f1c32510ccf9d6623dc903a13a6788fd49a3d07` |

上一批源提交 `695497c29ebc3d4f486a7674fb4838f811842685` 的 [Linux CI](https://github.com/full-stack-plugins/codeguard/actions/runs/37159461269) 已成功。该结果适用于那个提交，不适用于本轮未提交的 grammar 修复草稿；当前扩展测试仍是 RED。

显式 OTP 28 全 37 例对照已退出 101：原生编译标签和单文件原生反馈的逐例断言均通过，最终仍因同样 10 个 WASM 差异失败（55.29 秒）。这不是原生对照验收通过；日志 `/tmp/codeguard-erlang-grammar-expanded-native-red.log` SHA-256 `10f9414353ad179f8663aaf21c1237a91b683a138b304c4deea35a6fe6aafb0e`。普通回归也失败，不把成功捕获已知缺陷当作修复。
