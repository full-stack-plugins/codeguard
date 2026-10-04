# C09/C10 原生配置观察及修复指引

对应现有 OpenSpec 4.1、4.2、4.7、5.10；继续使用 `introduce-rust-codeguard-cli`，不是第二份需求或计划。

当前源码命令：

```bash
codeguard config validate . --format json
codeguard config explain . --format json
codeguard config explain .
```

## 实际行为

旧入口仅解释根级旧配置/工具锁/候选策略。当前 0.3 协议增加 `project_configuration`，通过既有 Rust discovery 静态发现服务按构建根投影原生检查器声明、配置来源及 SHA-256、原因和下一步。Maven P3C/PMD/Javadoc/Checkstyle/SpotBugs/依赖检查，Python Ruff/依赖输入和 Node ESLint/npm audit 沿用现有探测范围，不从配置文件存在推导规则已生效或检测已执行。

`configured` 仅代表当前静态可确认的声明。P3C 必须有明确制品、规则集及 `skipPmdError=false`；缺错误处理声明保持 `unknown`。坏 Ruff TOML 为 `invalid`，无配置为 `missing`。动态 ESLint 保持 `unknown`，即使脚本含写文件代码也不运行。Python 纯源码目录没有清单，不补造构建根；检查器仍保留自身根归属。

Human 输出最多 12 行，显示构建根、检查器、状态、来源、原因和准备动作；JSON 最多 64 个构建根、256 行检查器、每个摘要表 256 项及每个诊断列表 32 项，并保留完整观察计数。投影截断或必要范围无法观察时，静态观察状态为 incomplete。完整仅描述静态遍历，不代表原生语义/规则/质量检查完整。

旧 `codeguard.json` 递归拒绝重复 JSON 键，避免最后一个 exclude 或 java.commands 掩盖前值；配置字节读取复用 runtime 的有界普通文件方法。本命令不运行检查器、不写工作台、规则或批准状态，固定 `effective_policy=null`、`quality_decision=not_evaluated`、退出 3。

## 实际报告

以下为测试进程实际输出的**节选**，不是可直接消费的完整协议；完整原始输出见[JSON 证据](evidence/config-native-observation-2026-10-04.json)，可按[0.3 schema](../../schemas/config-inspection-v0.3.schema.json) 验证。来源摘要保存在原件中，没有填充占位 SHA。

```json
{
  "schema_version": "0.3.0",
  "report_type": "config_inspection",
  "operation": "explain",
  "exit_code": 3,
  "project_configuration": {
    "basis": "static_project_discovery",
    "observation_status": "complete",
    "native_execution": "not_run",
    "effective_rules": "unresolved",
    "suppressions": "unresolved",
    "truncated": false,
    "build_root_count": 2,
    "checker_count": 11,
    "checker_configurations": [
      {
        "build_root": "java",
        "category": "lint",
        "checker_id": "java.maven.p3c",
        "configuration": "configured",
        "configuration_ref": "java/pom.xml",
        "next_action": "运行原生 Maven PMD 并核对 P3C 规则加载、目标覆盖和诊断",
        "reason": "p3c_artifact_ruleset_and_error_policy_declared"
      },
      {
        "build_root": "python",
        "category": "lint",
        "checker_id": "python.ruff",
        "configuration": "configured",
        "configuration_ref": "python/.ruff.toml",
        "next_action": "用原生 Ruff 核对生效规则、扫描范围和诊断",
        "reason": "explicit_ruff_lint_configuration"
      },
      {
        "build_root": "node",
        "category": "lint",
        "checker_id": "node.eslint",
        "configuration": "unknown",
        "configuration_ref": "node/eslint.config.js",
        "next_action": "确认项目原 ESLint 版本、调用参数及选中配置；用原生工具核验逐文件规则、parser/插件与导入闭包，不套用默认规则或自动迁移配置",
        "reason": "eslint_dynamic_configuration_not_evaluated"
      }
    ]
  },
  "effective_policy": null,
  "quality_decision": "not_evaluated"
}
```

## TDD 与范围

首轮新入口测试 6 项均失败，报告没有原生观察，旧 JSON 还接受重复键。接线后一个测试仍失败：正例 POM 漏写 skipPmdError，而发现器正确返回 unknown。修正正例，同时追加缺声明保持 unknown 的反例，没有放宽检查器判定。终端来源/原因/下一步断言又先失败，再补反馈；不把测试夹具错误当作产品缺陷修复。

目标回归 5 组、58 passed/0 failed/0 ignored，含新增 7 项：多生态来源、动态配置不执行、280 行截断、坏/缺 Ruff、空/不存在范围、重复旧字段、P3C 未解析错误策略及终端动作。各行为有重叠测试，不当作 58 项新功能。

## 剩余验收

原生 effective model、规则启用与 suppression 差异、受保护策略来源、全生态配置发现及完整质量门禁仍缺；4.1/4.2/4.7/5.10 不因此勾选完成。发现服务仍有 100000 条目上限，尚未证明所有文件系统 I/O 的硬截止时间。公开 npm 0.1.4 尚未包含这批扩展。

默认 workspace 回归已终态退出 0：214 组、1199 passed/0 failed/109 ignored；109 条件项未执行，不算原生或平台验收。全工作区 all-targets、wasm-precheck Clippy -D warnings 通过；fmt、分层、OpenSpec strict、665 条本轮修改文档本地链接通过。201 份 schema 元定义、实际 0.3 原始输出、六组计数一致性及 12 类伪造/矛盾反例通过；旧 0.2 schema 与 HEAD 字节一致，历史 0.2 结构仍可验证。实际默认二进制的 human 来源/动作和 explain/范围阻塞 validate JSON 也已核验，前后项目文件字节一致。

没有更换 grammar、安装工具或改插件锁；原有 Erlang 37 例 RED 草稿保持未提交。当前扩展仍非完整规则/suppression/批准来源解析，父任务及总体目标继续未完成。远端 CI 按新提交另行核验，不使用旧提交证明当前批次。

| 日志 | SHA-256 |
| --- | --- |
| `/tmp/codeguard-config-native-observation-red.log` | `fee4236bd5212e064859391bac92768be273b0fdf96c1ede6ef0b652518c351c` |
| `/tmp/codeguard-config-native-human-red.log` | `dced9668558b16f3bdaaaaaa368d61a2e53d00b0f5682cf551ee856e02c90320` |
| `/tmp/codeguard-config-native-observation-green.log` | `f386065972f680503170db1d0920eae792d5e30fd1b900e67177f8e71f2fcb93` |
| `/tmp/codeguard-config-native-default-workspace.log` | `c2d986a26cb78fbe3f15c173c993f9a01743c2f1958b263cc3c2a0f80a542c8f` |
| `/tmp/codeguard-config-native-clippy.log` | `0ae4dc03be4ec3dbc4e8a22a97ebe531ab3f08fe08b65412a1b6f490d83fd832` |
| `/tmp/codeguard-config-native-schema.log` | `fa6b4779e684c1e25fdc5bce220a9123aaa3cc459e97480af16ad0148bb70cd0` |
| `/tmp/codeguard-config-native-public-cli.log` | `ce830e5c1b0b9f1ae53290c982b42ad9147e4cf006c3d0cd3bec39f63d45e9d4` |
| `/tmp/codeguard-config-native-links.log` | `de3ed6175700c6af3b427155f494c1757e454770757236276827ff9c81802c17` |
| `/tmp/codeguard-config-native-openspec.log` | `cf3ec12601576e4a3dd335699d867edc30029dd07ee9fdcd6396379524b1fa4b` |
| `/tmp/codeguard-config-native-layering.log` | `280c626d05d03e0944fca548fce5a1e9039200ab2cdd1f3d68f64c12e236de36` |
| `/tmp/codeguard-config-native-fmt.log` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
