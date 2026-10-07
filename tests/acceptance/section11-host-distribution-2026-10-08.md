# Section 11 宿主与分发验收

> 日期：2026-10-08；OpenSpec 11.1-11.10

## 验收标准覆盖

| 任务 | 标准 | 测试 | 状态 |
|---|---|---|---|
| 11.1 | 候选平台二进制构建 | binary_build_smoke | ✅ |
| 11.2 | 插件 runtime lock 和原子下载 | plugin_runtime_lock_atomic_download | ✅ |
| 11.3 | MCP 相同核心 API | mcp_core_api_versioned | ✅ |
| 11.4 | 宿主入口 hooks/__protocol__ | host_entry_hooks_protocol | ✅ |
| 11.5 | Git pre-commit/pre-push 和 CI | git_hook_ci_entry | ✅ |
| 11.8 | macOS arm64 候选制品 | macos_arm64_artifact | ✅ |
| 11.9 | macOS x86_64 候选制品 | macos_x86_64_artifact | ✅ |
| 11.10 | Linux x86_64 候选制品 | linux_x86_64_artifact | ✅ |

## 测试结果

| 测试 | 说明 | 状态 |
|---|---|---|
| binary_build_smoke | 二进制构建 smoke | ✅ |
| plugin_runtime_lock_atomic_download | 插件锁原子下载 | ✅ |
| mcp_core_api_versioned | MCP 版本化 API | ✅ |
| host_entry_hooks_protocol | 宿主入口协议 | ✅ |
| git_hook_ci_entry | Git hook/CI 入口 | ✅ |
| macos_arm64_artifact | macOS arm64 制品 | ✅ |
| macos_x86_64_artifact | macOS x86_64 制品 | ✅ |
| linux_x86_64_artifact | Linux x86_64 制品 | ✅ |
| distribution_signature_verification | 分发签名验证 | ✅ |
| archive_bundle_publish | 归档包发布 | ✅ |
| **总计** | | **10/10** |

## 模块测试覆盖

| 模块 | 测试数 | 状态 |
|---|---|---|
| distribution_installer | 5 | ✅ |
| archive_bundle | 5 | ✅ |
| mcp_integration | 3 | ✅ |
| **总计** | **13** | ✅ |

## Section 11 完成状态

- ✅ 11.1-11.5, 11.8-11.10: 8/10 完成
- ⏳ 11.6/11.7: 需外部环境验证

## 结论

Section 11 宿主与分发核心能力已验证。
二进制构建、插件锁、MCP、宿主入口、平台制品均有测试覆盖。
