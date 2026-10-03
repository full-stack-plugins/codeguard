# Erlang OTP 28 原生优先单文件语法检查

日期：2026-10-04。对应既有 OpenSpec 8.134、14.5–14.9、14.17、14.19 的局部实现；父任务未完成。

## 行为与边界

源码构建支持：

```bash
codeguard lint erlang sample.erl --erl-tool /absolute/path/to/erl --timeout 10s --format=json
codeguard lint erlang sample.erl --format=json
```

第一条优先通过 Rust 受控进程调用 OTP 28 `io:scan_erl_form` 和 `erl_parse:parse_form`，从 stdin 读取同一份有界 UTF-8 字节。固定 eval 不插入用户源码；源码始终是解析数据，不执行模块函数。使用固定 cwd `/`、`-no_dot_erlang`、清空的环境和有界 scheduler，不加载项目 `.erlang` 或 cwd 内 `.beam`，也不运行编译器、预处理或 parse_transform。声明 parse_transform 的合法 forms 可以解析，但该结果不代表 transform、编译或项目检查通过。

原生能识别当前固定 WASM 漏掉的最终句点：`-module(sample).\nf() -> ok\n` 报告第 2 行第 8 列 `erlang.syntax.error`。正常原生退出仍检查每项扫描/解析结果，不直接当作通过。原生诊断不被 WASM 覆盖；原生版本不合、无法启动、超时、输出或清理失败、重复 JSON 键、越界坐标、源码/工具变更均保持具体阻塞。显式工具无效时不悄悄转到 PATH 或 WASM。

宏和预处理指令按原生 token 检测，注释或字符串中的同名文本不应构成指令。涉及 `?MACRO`、define/include/条件编译的源码标为 `erlang_preprocessing_unresolved`，不把未展开 token 流的解析错误当作已确认源码违规。当前没有展开配置、include 路径或项目构建身份。

第二条未提供原生工具；WASM 特性构建可运行固定 Erlang 候选，并显示具体已知漏检和 `--erl-tool` 下一步。没有该特性的构建给出 `wasm_feature_not_built`。两条路径均整体退出 3、`coverage_proven=false`、`authority=local_unverified`、`delivery_decision=not_evaluated`。整体覆盖范围是单文件未预处理 forms，不是完整 lint；取消保留退出 130。

## 测试证据

红测：原实现不识别 `lint erlang`，四项新行为返回用法错误 2；日志 `/tmp/codeguard-erlang-native-first-red.log`：1 passed、4 failed，失败来自所需功能不存在。

当前受控入口回归：`erlang_lint_cli` 11 passed、0 failed、1 ignored。覆盖原生优先/诊断保留、缺工具固定候选、显式故障无替换、宏未完成、非法参数不启动、未知协议/重复键/坐标拒绝、工具/源码变更失效、共同截止时间、错误版本、真实 Ctrl-C 130/子孙进程回收，以及 human 首先显示原生正常结果而不是通用失败标题。原生工具用例明确隔离，未用受控替身证明原生正确性。

显式本机原生对照：既有 OTP 28 `erlc` 给 13 份源码独立标签（8 合法、5 非法）；新 `lint erlang --erl-tool` 对全部同字节样例均分类一致，含最终句点漏检反例，且不启动 WASM；最终显式对照 1 passed、0 failed，用时 23.66 秒。固定 WASM 仍为原始 12/13 一致，漏检未被删除或改成已修 grammar。运行命令：

```bash
CODEGUARD_ERLC_BIN=/opt/homebrew/bin/erlc CODEGUARD_ERL_BIN=/opt/homebrew/bin/erl \
cargo test --locked --offline -p codeguard-cli --features wasm-precheck \
  --test erlang_native_differential -- --ignored --nocapture
CODEGUARD_ERL_BIN=/opt/homebrew/bin/erl \
cargo test --locked --offline -p codeguard-cli --features wasm-precheck \
  --test erlang_lint_cli -- --ignored --nocapture
```

后一真实工具用例 1 passed、0 failed，7 份额外输入覆盖无末尾换行、Unicode、宏定义、外部宏、宏字符串拼接、条件编译和声明 parse_transform。测试在源码父目录放置写文件的 `.erlang`，检查后无副作用，原字节未变。

四份真实 CLI JSON（原生非法、原生正常、预处理未知、缺原生候选）通过独立 Draft 2020-12 [报告 Schema](../../schemas/erlang-lint-feedback-v0.1.schema.json)；12 个伪造整体 clean/allow/完整覆盖的变体被拒。三次当前小样例原生调用约 193–202ms，缺工具候选调用约 869ms；不是多语言、跨机器或稳定性能预算。

## 未验收范围

原生自动发现、项目多模块、预处理/include/依赖图、comments/CVE/security、稳定任务同步与能力匹配关闭、真实宿主、其它 OTP/平台、系统精度语料、全 I/O 硬预算及新 npm 发行仍缺。工具摘要绑定显式 launcher，不证明完整 OTP runtime 和标准库供应链；该原生局部观察不能用于可信关闭或签发项目门禁。当前发布的 npm 0.1.4 不含本轮新命令。

验证中的失败保留：Ctrl-C 用例初次未观察到原生就绪标记，原因是测试脚本写成 macOS 不存在的 `/bin/touch`；已纠正为现有 `/usr/bin/touch`，完整目标文件重新执行 11 passed、0 failed、1 ignored。日志 `/tmp/codeguard-erlang-final-guard-suite.log` 与 `/tmp/codeguard-erlang-final-guard-suite-corrected.log` 区分失败和修正后结果。human 反馈先红后绿，日志 `/tmp/codeguard-erlang-human-red.log`；没有把错误日志计作通过。


本轮最终验证（2026-10-04）：默认全工作区全目标 1132 passed、0 failed、106 ignored，204 个结果组，退出 0；最终 Erlang/库存特性目标 13 passed、0 failed、1 ignored；显式 OTP 28 的 13 例编译器对照与 7 例预处理/启动边界分别各 1 passed，显式 Swift 6.4 对照 1 passed（12 可判定一致、1 未解析）。CLI 全目标 WASM Clippy -D warnings、fmt、分层、OpenSpec strict、全部 schema 元定义、4 份真实 Erlang 报告/12 个伪造通过反例、2 份中英文完整示例及文档链接校验通过。工具依赖的忽略项没有当作通过，父任务未勾选。最终日志：`/tmp/codeguard-erlang-swift-workspace-final.log` SHA-256 `9d2d0b3a4291478b2acdd6fc31410c643081e545b46ef0f5cda349491ad35ad3`；`/tmp/codeguard-erlang-final-guard-suite-corrected.log` SHA-256 `750a219d900646abaa713ceb9c2de4f80378909205a8c85877b9b372af2fdd44`。远端 CI 须按本轮新提交独立核验；公开 npm/插件版本未改变。
