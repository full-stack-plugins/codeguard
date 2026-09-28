# Rust 规格与文档迁移记录

2026-09-28，按用户指示将 codeguard-plugin 中 11 份 `docs/rust-cli/` 文档和完整 `introduce-rust-codeguard-cli` change 移入本仓。同名 change 延续，没有另建第二份任务。原文件 SHA-256 记录于 [清单](migration-manifest.json)，包括迁移前的未提交验收更新及新增 syntax-precheck。

- 本仓拥有 Rust 规范、实施任务及验收；插件仅保留导航页。插件旧运行时 `openspec/specs`、其他 change、历史 CHANGELOG 仍归插件，不迁走。
- 新仓没有插件旧规格基线，因此将迁入 delta 的 MODIFIED 分组规范化为 ADDED，保留全部 requirement 标题、正文和场景；后续不能依赖不存在的旧基线进行 sync。legacy 条款仅描述兼容适用边界，不许可 Rust fail-open。
- 内部文档链接保留；引用插件源码及旧架构的链接改为原插件 GitHub 地址。历史验收中的 `codeguard-cli/` 是重命名前目录，不是第二个现存工程。
- 原勾选状态与历史日志保留；新增有界实现子任务，宽范围父任务仍未完成。WASM 独立 S14 全部待办。
- 不执行 sync/archive、不实现运行时、不安装或发布。迁移为本地工作树修改，远程导航待后续提交推送后生效。
