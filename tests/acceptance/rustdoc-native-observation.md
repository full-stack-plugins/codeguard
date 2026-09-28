# Rustdoc 原生机器流适配与重放

对应OpenSpec native-tool-adapters / Native evidence SHALL be interpreted per tool contract，任务7.1。Cargo原生文档入口及机器报告参数依据[Cargo rustdoc官方命令](https://doc.rust-lang.org/cargo/commands/cargo-rustdoc.html)，文档规则依据[Rustdoc官方lint目录](https://doc.rust-lang.org/rustdoc/lints.html)。此局部重放选择库目标，不宣称全部项目构建组合已检查。

新增独立parse_cargo_rustdoc_json，避免Clippy解析器丢弃非clippy规则。仅映射missing_docs与rustdoc::broken_intra_doc_links；原生规则、warning/error、唯一主定位及package/manifest/target保留用于后续归属核验，不保存自由文本诊断作为指令。16MiB总预算、单行1MiB、100000行预算；重复键递归拒绝，原生成功结束必须存在且后面不能再有事件。未知警告、编译错误、非UTF8、异常结束或歧义定位为未完成，已观察发现保留供调查，不签发覆盖或门禁。

TDD初态：新契约测试因缺少parse_cargo_rustdoc_json导出编译失败；没有用空stub充数。最小实现后cargo_rustdoc_contract 7项通过，覆盖规则/目标身份、无原始消息、编译错/未知警告分流、部分流、重复键/结束后事件、歧义/零定位、缺目标身份和损坏/未知/空诊断流。

真实验收：显式CODEGUARD_CARGO_BIN为本机stable-aarch64-apple-darwin/bin/cargo（Rust1.98.1），执行cargo test -p codeguard-cli --test rustdoc_native_replay -- --ignored，实际1项通过（1.95秒）。经共享Rust runtime执行cargo rustdoc --lib --offline --message-format=json -- --warn missing_docs --warn rustdoc::broken_intra_doc_links，五种不同原生输入分别验证：缺文档产生原生规则、正常文档无诊断、坏文档链接保留规则、编译错误不伪装文档违规、源码allow可使缺文档诊断消失。每轮独立target目录，源码字节保持不变，无安装/联网；原生抑制的零诊断不等于代码修复或批准白名单。

6项crate依赖边界回归通过；native重放归CLI层，adapters不依赖runtime。相关adapters/CLI两项测试目标的Clippy -D warnings及fmt通过。此为14项不同测试（7协议、6边界、1原生重放），不是五个独立测试或全工作区测试。

待实施：公开comments rust入口、共享命令计划、项目生效规则及源集/目标归属、工具/输入前后字节绑定、稳定任务及原工具复检、抑制对照、完整workspace/features/targets、可信政策、跨平台与宿主。纯解析器不校验实际进程退出或归属，调用方必须完成这些核验；7.1与完整计划保持未完成。
