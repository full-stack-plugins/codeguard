# Rustdoc 强制对照证据导入验收

2026-09-27；OpenSpec introduce-rust-codeguard-cli / native-tool-adapters。7.1/9.7仍未完成。

问题：旧复检导入只严格核验普通观察，封套内强制告警指纹损坏或重复输入归属仍可被消费。TDD反例先失败：预期两份坏报告被拒，实际failed_reports=0。

修正：封套顶层及输入归属严格核对，重复路径即拒绝；稳定输入绑定每个诊断源/目标和清单/锁。两轮观察分别通过同一严格原生范围/指纹解析；强制报告保留原始argv和证据，内部投影仅用于复用身份核验，不改变报告、规则或处置。输入不稳定或当前失配只生成准备阻塞，不生成源码修复任务。

反例包含三种变体：伪造强制指纹、重复输入归属、不稳定封套。前两种拒绝导入；第三种生成一项重扫准备任务，new_findings=0。相关普通文档CLI13、同步15、next6，共34项通过；3项默认忽略不计通过。真实Cargo CLI独立1项通过，14.49秒，含文档修复复检、真实allow/force-warn对照、中文链接。相关Clippy -D warnings退出0，7.18秒。真实任务报告及重复输入schema反例结果见插件verification.md；schema不能替代原生指纹和字段归属的Rust核验。

复跑：cargo test -p codeguard-cli --test rust_comments_cli --test work_sync_contract --test next_command_contract。指定既有CODEGUARD_CARGO_BIN后cargo test -p codeguard-cli --test rust_comments_cli -- --ignored。未安装工具；独立Python只校验协议，不进入产品路径。

未完成：正式批准/任务关闭复发、完整原配置/构建组合/符号归属、可信工具规则和所有语言/宿主。没有全工作区回归，不能以此次修正宣称完整计划完成。
