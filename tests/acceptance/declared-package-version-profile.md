# Maven/Cargo 直接包版本画像验收

范围：OpenSpec introduce-rust-codeguard-cli 9.16/9.17 的局部实现，不代表完整项目画像验收。

- 同一批初始清单字节同时提供 package_declared_versions 与 manifest_sha256；Maven version 不要求完整 group/artifact 坐标，不能从父项目或 profile 获取。只观察有界直接字面值，不声称验证 Maven 生效版本语义。
- Cargo package.version 通过缓存依赖 semver 1.0.28 校验，保留原始预发布/构建标识。workspace 继承、错类型、非法版本保持 unknown；不从 workspace.package 猜测。
- semver 仅登记为 adapters 的普通解析依赖；依赖边界反例继续拒绝 core/runtime/CLI 引用及 adapter 构建依赖，不改变核心层方向。
- 包版本独立于 language_targets、汇总 declared_version 和 installed_version，后两项在未探测时仍为空。
- 清单版本变化刷新画像及摘要，保留 tasks 人工备注；重复初始化幂等，不执行项目 Cargo/Maven、构建脚本或产生 Cargo.lock/target。

行为缺失时，Maven/Cargo 混合项目用例先以 Null 与声明版本不相等失败；实现后通过。回归入口：

```bash
cargo test -p codeguard-adapters --test package_version_contract --test cargo_module_model_contract --test maven_module_model_contract --offline
cargo test -p codeguard-cli --test init_command_contract --offline
```

模型反例覆盖变量、父/profile/workspace 继承、重复/嵌套/分段 XML、非法字面值、非 SemVer 和错误类型。CLI 覆盖两个生态共存、精确字节绑定、未知原因及刷新保留工作记录。

未完成：有效构建模型、解析依赖版本、本机版本探测、其它生态完整画像及可信准备任务。没有执行全语言或宿主端到端验收，9.16/9.17 仍不勾选。
