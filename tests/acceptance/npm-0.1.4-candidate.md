# npm 0.1.4 候选公开发行验收

日期：2026-10-04。关联 OpenSpec 13.4、14.18、11.17；父任务保持未完成。

## 公开身份

- 包：`@partme.ai/codeguard@0.1.4`，npm `latest=0.1.4`，仅 `darwin/arm64`。
- 干净构建来源：`1cd458f6e01a44a74388243e964e3f45290ac18e`；四个 workspace crate 与 Cargo.lock 版本一致。
- Tarball SHA-256：`a0d605f4a102e78d42766afaef61c6bd002556ccc5d16881c7553a91d6e20c80`；字节数 12280485。
- 程序 SHA-256：`34c2490ff4ccaa04bce9e91efd6ccabff6d679e97b08966b1edba9b6aec37266`；字节数 100531536。
- npm integrity：`sha512-99VQcM0JtasdqNW3gYoyMqbBUOQ1MXQJrncAIfnwz1UZOmyUbwlKKEcGsfi5ytlSl6JlgtehpX1+LAGPazpzAw==`。
- 精确 38 个普通包成员、32 份许可证；公开打包器核对全部固定 grammar 及许可证身份。
- [GitHub prerelease v0.1.4](https://github.com/full-stack-plugins/codeguard/releases/tag/v0.1.4) 的 tag 精确指向来源提交；上传 tarball 的 GitHub asset digest 与上列 SHA-256 相同。

## 实际验证

1. `npm publish --access public --tag latest --ignore-scripts` 成功；注册表下载字节与本地公开 tarball 一致。
2. 全新 npm 缓存的注册表 `npx --yes @partme.ai/codeguard@0.1.4 --version --format=json` 退出 0，报告版本、macOS arm64、来源提交正确；`grammar status` 退出 3，32 candidates、0 qualified/released、交付未评估。
3. 同一 release binary 的私有包顺序测试：4 passed、0 failed、0 skipped，包含无 WASM 构建拒绝、Zig/Dart、全部 32 grammar、编辑任务与复检。
4. 实际公开 tarball 另经离线 npm 安装：init → Zig 编辑 → 重复编辑同任务 → **真实 Zig 0.16.0** 原生失败 → next → 修改源码使旧位置失效 → 原生零诊断仍 open/unverified_policy → 复发仍同任务。Claude 形状输出含真实 task ID 和 task show 指引、限 1200 字符、不回显源码输入；此为直接事件重放，不是宿主自动触发证明。
5. 上一步 9 份版本化真实报告通过封闭 JSON Schema。库存 `grammar_coverage_inventory` 1.1.0 尚缺输出 schema，仅断言库存与未验收状态；Claude 未版本化宿主形状以工作流断言核验，不冒充全部 11 份 schema 通过。
6. [相同源码 Linux CI](https://github.com/full-stack-plugins/codeguard/actions/runs/37149050177) 全绿：WASM 阶段 160 passed/0 failed/8 ignored；工作区阶段 1124 passed/0 failed/105 ignored；npm 包 4 passed。不同阶段有重叠，不合计为独立测试数。
7. 前序 `a688292` 的 Linux SDK 测试因测试宿主制品超过 256 MiB 失败；后续仅去除测试调试符号、保留产品上限与 debug assertions。失败保留，修正提交和本发行来源分别有真实成功 CI，不把旧失败抹掉。

## 发行范围

本包新增编辑、稳定确认任务、原生复检和下一步反馈；不提供项目自批的 SDK 策略入口，普通 CLI 零诊断不能关闭任务。源码 SDK 的限定 Zig 可信关闭仍需独立受保护宿主。全部 32 grammar 未完成语言精度验收，已知反例不因发布消失。插件 lock/default Hook 升级是插件仓的独立工作；多平台、实际 Claude/Codex/Gemini 接线、完整 native-first、白名单权威及完整交付门禁仍缺。发行候选不等于 OpenSpec 整体完成。

## 日志身份

- `/tmp/codeguard-1cd458f-ci-success.log`：SHA-256 `e9c02c25750448a42eb1ee13371badc2b7885dd3d821cb7f9bdc1ebda8efb755`。
- `/tmp/codeguard-0.1.4-package-sequential-final.log`：SHA-256 `fdb843cdba01f0548f3a389db547bb102e84c49e011338d3b35202578c47d31f`。
- `/tmp/codeguard-0.1.4-public-schema.log`：SHA-256 `d2f9d7b03844ffa39f094b42aa57d6daa62f385b44a40619e5b8a7260c912973`。


后续源码补充：同一已发布程序的库存输出现已通过 [新增 1.1.0 schema](../../schemas/grammar-inventory-v1.1.schema.json)；[开发验收](grammar-inventory-schema.md)含 4 项正反例。发行当天“库存缺 schema”的历史证据不改写，程序字节与 tag 未变。
