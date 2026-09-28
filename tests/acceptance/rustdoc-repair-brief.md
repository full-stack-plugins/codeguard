# Rustdoc 逐问题修复简报验收

2026-09-27；OpenSpec introduce-rust-codeguard-cli / native-tool-adapters，任务7.1、9.7仍未完成。

报告0.3按原生missing_docs或broken_intra_doc_links给证据、规则、范围、步骤、复检、历史状态及关闭条件。源摘要/范围来自本轮已核对发现；原工具argv固定，不从原生消息生成执行指令。只在本轮局部完整且身份唯一时给目标文件的修复范围；输入变化、歧义及其它未完成状态给调查/重新检查，允许路径为空。历史未接通明确标注not_integrated；不以空数组证明没有失败，不创建正式任务或批准例外。

TDD：新增简报与输入变化反例在缺实现时两项失败。实现后rust_comments_cli七项普通测试、rustdoc_identity_contract三项通过；原生CLI忽略用例通过显式既有Cargo独立运行，4.34秒，包括missing docs、修复后零诊断、中文坏链接及对应简报。相关CLI Clippy -D warnings退出0，5.77秒。真实原生0.3报告通过Draft202012；删除七项信息字段的七个变体和未完成保留修复指引的变体被拒。历史0.2 schema按原字节保留。

复跑：cargo test -p codeguard-cli --test rust_comments_cli --test rustdoc_identity_contract；原生用例需显式CODEGUARD_CARGO_BIN并加-- --ignored。没有安装工具。协议校验Python仅独立验收，产品链全Rust。

未完成：持久任务同步、原生task verify/抑制对照、历史尝试、关闭重开、check all调度、原配置和全部features/targets/workspace、可信工具/规则/批准及跨平台宿主。局部简报不是完整RepairBrief工作流验收。本轮没有全工作区回归。
