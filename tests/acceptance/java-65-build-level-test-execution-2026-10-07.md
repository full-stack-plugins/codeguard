# Java 6.5 build 等级和测试执行声明验收

> 日期：2026-10-07；OpenSpec 6.5

## 验收标准覆盖

| 标准 | 证据 | 状态 |
|---|---|---|
| 默认静态构建不谎称测试通过 | rust_build_command.rs: test_execution=false | ✅ |
| 要求测试的策略不可自动跳过 | policy_repair_test.rs: test_policy_weakening | ✅ |

## 测试结果

| 测试 | 说明 | 状态 |
|---|---|---|
| default_static_build_never_claims_test_passed | 静态构建不声称测试通过 | ✅ |
| static_build_with_findings_still_reports_test_not_executed | 有 findings 也不声称测试执行 | ✅ |
| policy_requiring_tests_cannot_be_auto_skipped | 要求测试的策略不可跳过 | ✅ |
| build_level_upgrade_requires_explicit_test_execution | build_level 升级需明确测试执行 | ✅ |
| static_build_reports_never_include_test_results | 静态构建报告无测试结果 | ✅ |
| **总计** | | **5/5** |

## 验证明细

### 默认静态构建不谎称测试通过 ✅
- `build_level: "type_check"` 表示静态类型检查
- `test_execution: false` 明确声明未执行测试
- `build_success` 只表示类型检查通过，不是测试通过
- work_sync 验证 `test_execution` 必须为 `false`

### 要求测试的策略不可自动跳过 ✅
- `test_policy_weakening` 检查 CLI/env/project 不能弱化策略
- 要求测试的策略必须显式执行测试
- `build_level` 升级到 `test_execution` 需明确执行测试

## 实现位置

- `crates/codeguard-cli/src/rust_build_command.rs:114` - build_level/test_execution 声明
- `crates/codeguard-cli/src/work_sync/rust_build_report.rs:116` - test_execution 验证
- `crates/codeguard-cli/src/policy_repair_test.rs:29` - 策略弱化防护

## 结论

6.5 build 等级和测试执行声明验收标准全部满足。
默认静态构建不谎称测试通过，要求测试的策略不可自动跳过。
