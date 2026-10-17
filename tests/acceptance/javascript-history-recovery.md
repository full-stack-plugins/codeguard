# JavaScript 历史确认恢复

对应 OpenSpec `introduce-rust-codeguard-cli` 9.7/9.10、14.8/14.10。独立 JavaScript 兜底的零候选分支原先将历史读者任何失败视为无任务；损坏 JSON 的公开反例先 RED，真实 CLI 输出推荐安装而隐藏待确认记录。修复后区分可核验的无同范围任务和无法核验的历史。

事实损坏、投影缺失、链接投影、伪造 closed、损坏工作区均保留 required 原生确认，工作台报告 incomplete、固定失败原因和空任务引用。父目录链接或非目录在读取任务前拒绝。恢复指引要求核对 findings/tasks/reports 和工作区身份，不从任务 Markdown 提取指令，不修改用户源码或伪造关闭证据。

正向对照：未绑定的零候选仍推荐；显式未初始化工作区要求 init；已初始化且确实无同范围历史仍推荐；有效开放历史仍复用原 ID。报告仍为 0.6，沿用现有封闭失败协议，不修改历史 schema 字节。

四组受影响 WASM 回归 35 passed/0 failed/0 ignored；最终新增对照后的本目标 6 passed/0 failed/0 ignored（与前述重复，不累加）。实际 JSON 校验见 `evidence/javascript-history-recovery-2026-10-06.json`。本批不启动原生 ESLint，不证明实际宿主自动反馈、正式关闭/重开、32语言精度或发行资格，父任务保持开放。受保护 Erlang 草稿未执行或提交。

最终实际反馈27份全部通过现有0.6封闭schema，8份失败反馈均保留required和空任务引用；366份schema元定义校验通过。WASM CLI全目标Clippy -D warnings退出0；OpenSpec strict、分层及diff检查通过。没有改动默认特性生产路径或既有协议文件。
