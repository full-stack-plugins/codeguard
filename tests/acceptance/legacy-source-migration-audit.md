# 最新旧源码差异核对验收

## 范围与事实源

OpenSpec1.1要求核对最新旧源码与57语言清单的差异，并建立保留正确行为/明确纠偏/legacy兼容表，每条差异关联spec和fixture。该任务是迁移审计；真正的六类别原生迁移、兼容运行时及宿主验收由S05–S12另行承担，不能以审计完成替代。

固定插件来源dec5f9d1361eefff493e120b4278939a243814a9，原设计基线03ebb24。bin/scripts/hooks共21个变化文件，含相对9e4adb1的14个新增变化；全部源码差异已阅读，830行可移植校验器也完整检查。Git对象源码前后SHA256及每条固定夹具摘要独立保存于[源码审计](evidence/legacy-source-audit-2026-10-06.json)，分类与规格引用见[双语表](../../docs/Codeguard-Legacy-Registry-Audit.zh_CN.md)。57语言全部字段对照另见[语言审计](legacy-registry-identity-audit.md)。

## 实际验证

旧插件源树干净且固定于dec5f9d。engine边界21项、skipGate安全8项、清单4项及提示入口5项，共38项Python回归通过；执行的是具名旧兼容测试，不将Python接回新CLI。可移植插件校验和旧架构校验通过。

Node runtime/lifecycle共6通过、2条件跳过。跳过的下载安装/实际固定公开程序场景不计通过；这些回归及本表不能代替真实智能体宿主自动触发验收。

Rust审计示例`audit_legacy_source`只读取固定Git对象，不执行旧检查命令；固定源码核对实际通过；示例两项测试显式包括ignored目标执行，2通过/0失败/0忽略，核对实际Git对象并拒绝漏项、重复项、源码摘要/夹具摘要篡改、未分类项及无效规格引用。首轮发现crate测试工作目录导致规格相对路径失配，改为从CARGO_MANIFEST_DIR解析真实仓库后通过；没有放宽不存在的规格引用。定向严格Clippy通过。

提交7fec0c9完整默认workspace/all-targets实际275组、1516通过/0失败/135条件忽略；本次新增审计示例单独验收，不混计。OpenSpec1.1的审计范围已具备证据；CI独立核验，总体目标仍开放。

## 共性纠偏及后续任务

- 旧通用rc=1推断finding：新工具必须解释原生报告，缺工具/坏报告保留未完成。规格native-tool-adapters；夹具`crates/codeguard-adapters/tests/checkstyle_failure_contract.rs`。真实全工具解析由S05–S08完成。
- 旧delta最多50文件/基线抵消：新完整交付覆盖与预算由execution-kernel、scan-scope-policy约束；夹具`crates/codeguard-cli/tests/check_all_partial_contract.rs`。完整门禁和所有命令的范围由S02/S03/S12验收。
- 旧skipGate/fail-open：仅具名legacy语义；新交付不消费本地豁免。规格hook-protocol/verdict-integrity；固定插件`tests/test_skip_gate_safety_scope.py`及新`legacy_v1_protocol_contract`。
- 旧Maven verify代替多类别质量：新原生义务分开准备和执行。规格language-gate-commands/native-tool-adapters；夹具`crates/codeguard-cli/tests/maven_probe_contract.rs`。S06多类别真实验收仍开放。
- 新增engine自报implemented/退出0：只是旧引擎观察，不是受保护交付权威。规格verdict-integrity；固定插件`tests/test_engine_boundary.py`。正式可信政策和来源由S04/S09完成。

这些表记录保留/纠偏/兼容的决定与已存在夹具，未把尚未完成的适配器、可信门禁、独立WASM精度或发行勾选。
