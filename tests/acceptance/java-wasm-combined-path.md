# Java WASM+原生联合路径全链路验收（2026-10-07）

> 对应 OpenSpec 15.2。验证 Java 语言的 WASM 语法候选与原生 lint/编译器联合路径，
> 覆盖配置发现、原生优先、缺工具初检、版本/方言兼容、精确定位、取消/超时及安装指引全链路。

## 测试结果

| 测试 | 覆盖维度 | 状态 |
|---|---|---|
| `wasm_candidates_are_not_confirmed_violations` | WASM 候选 ≠ 已确认违规 | ✅ |
| `unverified_grammar_does_not_claim_clean` | grammar 未验证不声称 clean | ✅ |
| `native_preferred_over_wasm` | 原生优先路由 | ✅ |
| `configuration_discovery_identifies_checker_status` | 配置发现（POM→checker 状态） | ✅ |
| `missing_tool_initial_check_gives_guidance` | 缺工具初检 + 安装指引 | ✅ |
| `version_dialect_compatibility_disclosed` | 版本/方言兼容性披露 | ✅ |
| `precise_positioning_with_grammar_identity` | 精确定位（byte_offset + grammar SHA-256） | ✅ |
| `execution_budget_has_timeout_config` | 取消/超时预算 | ✅ |
| `installation_guidance_in_category_candidates` | 六类别 gap 指引 | ✅ |
| **总计** | **9/9 全链路** | **✅** |

## 全链路验证详情

### 1. 配置发现
`codeguard detect --format=json` 正确识别 Maven POM 中声明的 checker：
- `java.maven.checkstyle` → `configured`（POM 中已声明）
- `java.maven.p3c`/`pmd`/`javadoc`/`dependency`/`dependency_check`/`findsecbugs` → `missing`
- 每个配置含 `next_action` 安装/配置指引

### 2. 原生优先
`check all` 在原生工具可用时优先使用原生；WASM 仅作候选回退。
`delivery_decision` 在 grammar 未完全验证时保持 `not_evaluated`/`incomplete`。

### 3. 缺工具初检
无原生 Java 工具时：
- `syntax_candidates.next_action` 给出具体指引
- `delivery_decision = incomplete`
- 退出码 = 3（未完成，不假通过）

### 4. 版本/方言兼容
- `known_limitations` 如实披露 Java 17 语法语料限制、版本/方言未验收
- `grammar_qualified = false`（grammar 未正式验收）

### 5. 精确定位
- `byte_offset`：字节级精确定位
- `grammar_sha256`：64 字符 SHA-256 grammar 身份绑定
- `path`：源文件路径

### 6. 取消/超时
- `execution_budget.timeout_ms`：超时限制
- `execution_budget.enforcement`：执行策略

### 7. 安装指引
六类别（lint/comments/dependencies/cve/security/build）各自 `capability_status` + `status`，
gap 状态含 `next_action` 或 `reason`。

## 复现命令

```bash
cargo test --package codeguard-cli --features wasm-precheck \
  --test java_wasm_combined_path --locked --offline
```

## 边界

- grammar 版本/方言仍为 `unqualified`，不代表 Java 17/21 全覆盖
- 正式 grammar 资格仍为 0/32
- 本验收只覆盖单文件/简单项目路径，多模块 Maven/Gradle 调度另行验收
- 宿主对话接入、可信关闭/复发、发行包实装仍未完成
