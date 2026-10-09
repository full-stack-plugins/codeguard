# 工程守卫需求、任务与验收追溯

日期：2026-10-09。共 50 条新增要求、78 项待实施任务、36 个跨仓端到端用例。此表仅引用原生账本，不保存第二份完成状态。每条正式 Requirement 自带 WHEN/THEN 场景；端到端用例补充跨组件验证，不替代局部场景。

## codeguard

| 要求 | 规范位置 | 实施任务 | 关联端到端用例 |
|---|---|---|---|
| EG-GATE-001 Unified action decision | [engineering-gate](../../openspec/changes/add-engineering-guard-core/specs/engineering-gate/spec.md) | EG-K07, EG-K08, EG-K12 | EG-AT-001, EG-AT-002, EG-AT-006, EG-AT-007 |
| EG-GATE-002 Independent authority | [engineering-gate](../../openspec/changes/add-engineering-guard-core/specs/engineering-gate/spec.md) | EG-K09, EG-K26 | EG-AT-004, EG-AT-014, EG-AT-019, EG-AT-034 |
| EG-GATE-003 Scoped reusable approvals | [engineering-gate](../../openspec/changes/add-engineering-guard-core/specs/engineering-gate/spec.md) | EG-K09 | EG-AT-004, EG-AT-014, EG-AT-019 |
| EG-GATE-004 Enforced execution | [engineering-gate](../../openspec/changes/add-engineering-guard-core/specs/engineering-gate/spec.md) | EG-K10, EG-K27 | EG-AT-014, EG-AT-017, EG-AT-035 |
| EG-GATE-005 Nonweakening exceptions | [engineering-gate](../../openspec/changes/add-engineering-guard-core/specs/engineering-gate/spec.md) | EG-K08, EG-K16 | EG-AT-002, EG-AT-003, EG-AT-007, EG-AT-028 |
| EG-GATE-006 Honest operating modes | [engineering-gate](../../openspec/changes/add-engineering-guard-core/specs/engineering-gate/spec.md) | EG-K10, EG-K12, EG-K19, EG-K20, EG-K28 | EG-AT-001, EG-AT-014, EG-AT-017, EG-AT-031, EG-AT-032 |
| EG-GATE-007 Recovery and compatibility | [engineering-gate](../../openspec/changes/add-engineering-guard-core/specs/engineering-gate/spec.md) | EG-K06, EG-K11, EG-K18 | EG-AT-007, EG-AT-010, EG-AT-012, EG-AT-030 |
| EG-EVI-001 Separate dimensions | [engineering-evidence](../../openspec/changes/add-engineering-guard-core/specs/engineering-evidence/spec.md) | EG-K06, EG-K17 | EG-AT-007, EG-AT-010, EG-AT-012 |
| EG-EVI-002 Exact subject | [engineering-evidence](../../openspec/changes/add-engineering-guard-core/specs/engineering-evidence/spec.md) | EG-K04 | EG-AT-010, EG-AT-011 |
| EG-EVI-003 Freshness and ordering | [engineering-evidence](../../openspec/changes/add-engineering-guard-core/specs/engineering-evidence/spec.md) | EG-K06 | EG-AT-007, EG-AT-010, EG-AT-012 |
| EG-EVI-004 Versioned projections | [engineering-evidence](../../openspec/changes/add-engineering-guard-core/specs/engineering-evidence/spec.md) | EG-K01, EG-K07, EG-K18, EG-K20 | EG-AT-006, EG-AT-030, EG-AT-031 |
| EG-EVI-005 Durable private evidence | [engineering-evidence](../../openspec/changes/add-engineering-guard-core/specs/engineering-evidence/spec.md) | EG-K05, EG-K26 | EG-AT-029, EG-AT-034 |
| EG-ARCH-001 System boundaries | [architecture-governance](../../openspec/changes/add-engineering-guard-core/specs/architecture-governance/spec.md) | EG-K12, EG-K13, EG-K15, EG-K17 | EG-AT-001, EG-AT-024, EG-AT-025 |
| EG-ARCH-002 Domain invariants | [architecture-governance](../../openspec/changes/add-engineering-guard-core/specs/architecture-governance/spec.md) | EG-K12, EG-K14 | EG-AT-001, EG-AT-024 |
| EG-ARCH-003 Design review separation | [architecture-governance](../../openspec/changes/add-engineering-guard-core/specs/architecture-governance/spec.md) | EG-K15, EG-K21, EG-K22, EG-K24 | EG-AT-025 |
| EG-ARCH-004 Semantic conflicts | [architecture-governance](../../openspec/changes/add-engineering-guard-core/specs/architecture-governance/spec.md) | EG-K21, EG-K23, EG-K24 | EG-AT-026, EG-AT-027 |
| EG-ARCH-005 Evolution and baselines | [architecture-governance](../../openspec/changes/add-engineering-guard-core/specs/architecture-governance/spec.md) | EG-K25, EG-K27 | EG-AT-033, EG-AT-035 |
| EG-CON-001 Approved snapshot | [engineering-contracts](../../openspec/changes/add-engineering-guard-core/specs/engineering-contracts/spec.md) | EG-K01, EG-K03 | EG-AT-002, EG-AT-009 |
| EG-CON-002 Rule composition | [engineering-contracts](../../openspec/changes/add-engineering-guard-core/specs/engineering-contracts/spec.md) | EG-K02 | EG-AT-008, EG-AT-036 |
| EG-CON-003 Frozen obligations | [engineering-contracts](../../openspec/changes/add-engineering-guard-core/specs/engineering-contracts/spec.md) | EG-K03 | EG-AT-002, EG-AT-009 |
| EG-CON-004 Contract evolution | [engineering-contracts](../../openspec/changes/add-engineering-guard-core/specs/engineering-contracts/spec.md) | EG-K03, EG-K16 | EG-AT-002, EG-AT-003, EG-AT-009, EG-AT-028 |
| EG-CON-005 Safe discovery | [engineering-contracts](../../openspec/changes/add-engineering-guard-core/specs/engineering-contracts/spec.md) | EG-K02, EG-K11 | EG-AT-008, EG-AT-036 |
## codegraph-plugin

| 要求 | 规范位置 | 实施任务 | 关联端到端用例 |
|---|---|---|---|
| EG-CGP-001 Client boundary | `openspec/changes/integrate-engineering-guard-facts/specs/engineering-fact-provider/spec.md` | EG-F01, EG-F08 | 正式规格局部场景；发布综合验收 |
| EG-CGP-002 Fact identity | `openspec/changes/integrate-engineering-guard-facts/specs/engineering-fact-provider/spec.md` | EG-F02, EG-F05, EG-F07 | EG-AT-023, EG-AT-036 |
| EG-CGP-003 Explicit indexing | `openspec/changes/integrate-engineering-guard-facts/specs/engineering-fact-provider/spec.md` | EG-F02 | EG-AT-023, EG-AT-036 |
| EG-CGP-004 Bounded capabilities | `openspec/changes/integrate-engineering-guard-facts/specs/engineering-fact-provider/spec.md` | EG-F03, EG-F05, EG-F06 | EG-AT-023 |
| EG-CGP-005 Lifecycle compatibility | `openspec/changes/integrate-engineering-guard-facts/specs/engineering-fact-provider/spec.md` | EG-F01, EG-F04, EG-F07, EG-F08 | 正式规格局部场景；发布综合验收 |
## codeguard-plugin

| 要求 | 规范位置 | 实施任务 | 关联端到端用例 |
|---|---|---|---|
| EG-CGH-001 Pinned bridge | `openspec/changes/integrate-engineering-guard-host/specs/engineering-host-bridge/spec.md` | EG-H01, EG-H04, EG-H06, EG-H10 | EG-AT-030 |
| EG-CGH-002 Correct event subject | `openspec/changes/integrate-engineering-guard-host/specs/engineering-host-bridge/spec.md` | EG-H02, EG-H06 | EG-AT-030 |
| EG-CGH-003 Singleflight interaction | `openspec/changes/integrate-engineering-guard-host/specs/engineering-host-bridge/spec.md` | EG-H03, EG-H09 | EG-AT-012, EG-AT-018 |
| EG-CGH-004 Nonmisleading feedback | `openspec/changes/integrate-engineering-guard-host/specs/engineering-host-bridge/spec.md` | EG-H02, EG-H04, EG-H05, EG-H09 | EG-AT-022 |
| EG-CGH-005 Host and distribution gates | `openspec/changes/integrate-engineering-guard-host/specs/engineering-host-bridge/spec.md` | EG-H07, EG-H08, EG-H10 | EG-AT-032 |
## codereview-plugin

| 要求 | 规范位置 | 实施任务 | 关联端到端用例 |
|---|---|---|---|
| EG-CR-001 Review is evidence | `openspec/changes/integrate-engineering-guard-review/specs/engineering-review-evidence/spec.md` | EG-R01, EG-R06, EG-R09 | EG-AT-005, EG-AT-022 |
| EG-CR-002 Consent compatibility | `openspec/changes/integrate-engineering-guard-review/specs/engineering-review-evidence/spec.md` | EG-R02, EG-R07, EG-R10 | EG-AT-019, EG-AT-020, EG-AT-021 |
| EG-CR-003 Subject exactness | `openspec/changes/integrate-engineering-guard-review/specs/engineering-review-evidence/spec.md` | EG-R03, EG-R07, EG-R09 | EG-AT-011 |
| EG-CR-004 Grounded architectural review | `openspec/changes/integrate-engineering-guard-review/specs/engineering-review-evidence/spec.md` | EG-R05, EG-R08 | EG-AT-027 |
| EG-CR-005 Safe coordinated delivery | `openspec/changes/integrate-engineering-guard-review/specs/engineering-review-evidence/spec.md` | EG-R01, EG-R04, EG-R06, EG-R10 | EG-AT-005, EG-AT-022 |
## flowguard-plugin

| 要求 | 规范位置 | 实施任务 | 关联端到端用例 |
|---|---|---|---|
| EG-WF-001 保留十阶段与原生事实源 | `docs/superpowers/specs/2026-09-23-flowguard-docs-ten-stage-governance.md` | EG-W01, EG-W07, EG-W12 | 正式规格局部场景；发布综合验收 |
| EG-WF-002 独立证据与实际候选 | `docs/superpowers/specs/2026-09-23-flowguard-docs-ten-stage-governance.md` | EG-W02, EG-W09 | EG-AT-005 |
| EG-WF-003 批准的来源与有效期 | `docs/superpowers/specs/2026-09-23-flowguard-docs-ten-stage-governance.md` | EG-W03, EG-W04, EG-W05, EG-W11 | EG-AT-004, EG-AT-018, EG-AT-020, EG-AT-034 |
| EG-WF-004 唯一流程判定与统一动作决策 | `docs/superpowers/specs/2026-09-23-flowguard-docs-ten-stage-governance.md` | EG-W03, EG-W05, EG-W07 | EG-AT-004 |
| EG-WF-005 跨插件协调与有界恢复 | `docs/superpowers/specs/2026-09-23-flowguard-docs-ten-stage-governance.md` | EG-W04, EG-W08 | EG-AT-018, EG-AT-020, EG-AT-032 |
| EG-WF-006 任务依赖与风险范围 | `docs/superpowers/specs/2026-09-23-flowguard-docs-ten-stage-governance.md` | EG-W06, EG-W09, EG-W10 | EG-AT-033 |
| EG-WF-007 历史与平台治理 | `docs/superpowers/specs/2026-09-23-flowguard-docs-ten-stage-governance.md` | EG-W10, EG-W11 | EG-AT-033, EG-AT-034 |
| EG-WF-008 宿主与受控门禁验收 | `docs/superpowers/specs/2026-09-23-flowguard-docs-ten-stage-governance.md` | EG-W05, EG-W08, EG-W12 | EG-AT-004, EG-AT-032 |
## gitflow-plugin

| 要求 | 规范位置 | 实施任务 | 关联端到端用例 |
|---|---|---|---|
| EG-GIT-001 Operation evidence | `openspec/changes/integrate-engineering-guard-git/specs/engineering-git-integration/spec.md` | EG-G01, EG-G10 | EG-AT-006 |
| EG-GIT-002 Change scope | `openspec/changes/integrate-engineering-guard-git/specs/engineering-git-integration/spec.md` | EG-G02, EG-G05 | EG-AT-002, EG-AT-017 |
| EG-GIT-003 Exact merge candidate | `openspec/changes/integrate-engineering-guard-git/specs/engineering-git-integration/spec.md` | EG-G03, EG-G04, EG-G09 | EG-AT-013, EG-AT-015, EG-AT-035 |
| EG-GIT-004 Independent CI | `openspec/changes/integrate-engineering-guard-git/specs/engineering-git-integration/spec.md` | EG-G05, EG-G06, EG-G07, EG-G10 | EG-AT-002, EG-AT-016, EG-AT-017 |
| EG-GIT-005 Safe execution and conflicts | `openspec/changes/integrate-engineering-guard-git/specs/engineering-git-integration/spec.md` | EG-G04, EG-G07, EG-G08, EG-G09 | EG-AT-013, EG-AT-015, EG-AT-016, EG-AT-026, EG-AT-035 |

## 使用规则

规范位置相对于本节仓库根；跨仓不依赖安装者具有兄弟目录。完整任务/依赖见各自 tasks 或 FlowGuard 原计划第 5 节。逐条验证在实施时保存实际测试名、命令、源码/输入指纹和报告引用；本表不填虚假的通过记录。
