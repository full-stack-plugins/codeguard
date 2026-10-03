# 离线 npm 包的编辑与修复任务链路

对应 OpenSpec `introduce-rust-codeguard-cli` 的 `binary-distribution` 安装后修复反馈场景，关联 11.17、13.4、14.18。此记录证明本机 macOS arm64 私有候选包的实际运行，不证明公开版本升级、默认插件或已安装 Claude 自动触发。

## 对照与结果

先从 npm 注册表获取不可变的公开 0.1.3 资产，核对插件 lock 的 tarball SHA-256 `37509cdd108fee631eebb9ebcd794a6e8d63652801b3522625d0909c3da2ae12` 及程序 SHA-256 `04bdea8c489693a8e97b451df8d0c2b7a89ba82372db5a4d1fed277ba19b0f4d`。仅提取经核对的普通程序成员。新安装链路测试在旧制品上真实失败：编辑事件的 `local_feedback` 没有语法任务。该 RED 用于证明已发布旧包缺少能力，不是当前源码新增缺陷。

当前源码 WASM 程序由真实 packer 生成私有 `@full-stack-plugins/codeguard` tarball，并在独立缓存经 `npm exec --offline --ignore-scripts` 安装调用。测试实际执行 init、编辑 Hook、重复编辑、Claude 形状保存事件、原生 task verify、next、修改源码后的 next、原生零诊断、WASM 零恢复与问题再次出现。重复扫描与问题再次出现保持同一真实任务 ID；原生证据指导修复；源码变化后旧位置清空；没有受保护策略时原生零诊断及 WASM 零恢复均不关闭首次 finding。

最终测试 1 passed、0 failed、0 skipped，62.4 秒。早先当前源码试跑因测试使用 task verify 不支持的等号形式 `--timeout=30s` 失败，测试已改为该命令现有的 `--timeout 30s`；这次试跑不计成功，也不计产品缺陷。成功运行导出的 9 份版本化真实报告通过对应 JSON Schema，拒绝伪造交付 allow、复检 resolved 及简报 resolved 三种变体。Claude 格式输出含真实任务 ID 和 task show 指引，限制 1200 字符，不回显输入中的源码标记；此证据仍是宿主形状重放，不是已安装宿主验收。

## 复现

```bash
cargo build --locked -p codeguard-cli --features wasm-precheck --offline
CODEGUARD_WASM_BIN="$PWD/target/debug/codeguard" \
node --test tests/npm_pack_repair.test.mjs
```

可以使用 `CODEGUARD_NPM_REPAIR_ARTIFACT=/absolute/path/report.json` 导出测试实际报告，默认不导出。测试临时项目与 npm 缓存运行后清除，不执行 npm publish 或全局安装。CI 顺序执行原有 32 grammar 包测试与本测试，避免共享 tarball 路径竞争；Linux 新增用例的远端结果须按对应提交核验。

## 范围限制

原生工具是受控 Zig 0.16.0 形状夹具，只证明 Node/Rust 安装后的协议编排；真实 Zig 与签名生命周期另见 [任务生命周期验收](task-resolution-lifecycle.md)。32 grammar 的全量包内执行另见 [WASM 包验收](npm-wasm-local-package.md)。公开 `@partme.ai/codeguard@0.1.3` 仍缺本批闭环，默认插件、可信策略来源、其它宿主/平台、完整门禁和语言精度未完成。11.17、13.4、14.18 父任务不勾选。

日志身份：
- `/tmp/codeguard-npm-repair-red.log`：SHA-256 `cfb132d2edf04d54187e44e3b81bb0ceaa30c60c3529f8f180c837662cc66134`。
- `/tmp/codeguard-npm-repair-green-final.log`：SHA-256 `4b0ccaa0327a61097d5efc6fea5a47aa55c78fbdb71d1ca5283fa5cfe0e62d98`。
- `/tmp/codeguard-npm-repair-schema-final.log`：SHA-256 `8e88893969dfb750249a73dcaa468d8ac478155b322b814fcbc4531d1edda045`。

Linux 前置 SDK 验收在提交 a688292 的 CI 37147462608 失败，npm 步骤被跳过。后续 CI 去掉测试制品调试符号并加入 256 MiB 大小断言，产品读取预算不放宽；本机同 profile SDK 9 passed/0 failed/1 ignored。新增 npm 链路及 Linux 修正都须按新提交等待远端结果，不能视为已通过。
