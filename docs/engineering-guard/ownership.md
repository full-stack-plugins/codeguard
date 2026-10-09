# 任务所有权与迁移

本表只记录目标实现 owner，不记录完成状态。当前唯一 tasks 文件仍为每条任务所在的原生账本；新仓尚未建立时使用 CodeGuard change 的临时账本。EG-M01 完成移交后更新链接并冻结旧位置，ID 不变。

| 任务 | 目标产品仓 | 范围 |
|---|---|---|
| EG-K01 | `guardcore` | 历史编号保留；按现行七项目边界实现 |
| EG-K02 | `guardcore` | 历史编号保留；按现行七项目边界实现 |
| EG-K03 | `guardcore` | 历史编号保留；按现行七项目边界实现 |
| EG-K04 | `guardcore` | 历史编号保留；按现行七项目边界实现 |
| EG-K05 | `guardcore` | 历史编号保留；按现行七项目边界实现 |
| EG-K06 | `guardcore` | 历史编号保留；按现行七项目边界实现 |
| EG-K07 | `codeguard` | 历史编号保留；按现行七项目边界实现 |
| EG-K08 | `guardcore` | 历史编号保留；按现行七项目边界实现 |
| EG-K09 | `guardcore` | 历史编号保留；按现行七项目边界实现 |
| EG-K10 | `guardcore` | 历史编号保留；按现行七项目边界实现 |
| EG-K11 | `guardcore` | 历史编号保留；按现行七项目边界实现 |
| EG-K12 | `flowguard` | 历史编号保留；按现行七项目边界实现 |
| EG-K13 | `archguard` | 历史编号保留；按现行七项目边界实现 |
| EG-K14 | `archguard` | 历史编号保留；按现行七项目边界实现 |
| EG-K15 | `archguard` | 历史编号保留；按现行七项目边界实现 |
| EG-K16 | `flowguard` | 历史编号保留；按现行七项目边界实现 |
| EG-K17 | `archguard` | 历史编号保留；按现行七项目边界实现 |
| EG-K18 | `guardcore` | 历史编号保留；按现行七项目边界实现 |
| EG-K19 | `guardcore` | 历史编号保留；按现行七项目边界实现 |
| EG-K20 | `guardcore` | 历史编号保留；按现行七项目边界实现 |
| EG-K21 | `archguard` | 历史编号保留；按现行七项目边界实现 |
| EG-K22 | `archguard` | 历史编号保留；按现行七项目边界实现 |
| EG-K23 | `gitguard` | 历史编号保留；按现行七项目边界实现 |
| EG-K24 | `archguard` | 历史编号保留；按现行七项目边界实现 |
| EG-K25 | `archguard` | 历史编号保留；按现行七项目边界实现 |
| EG-K26 | `guardcore` | 历史编号保留；按现行七项目边界实现 |
| EG-K27 | `flowguard` | 历史编号保留；按现行七项目边界实现 |
| EG-K28 | `flowguard` | 历史编号保留；按现行七项目边界实现 |
| EG-M01 | `guardcore` | 七项目新增工作 |
| EG-A01 | `archguard` | 七项目新增工作 |
| EG-A02 | `archguard` | 七项目新增工作 |
| EG-Q01 | `codeguard` | 七项目新增工作 |
| EG-Q02 | `codeguard` | 七项目新增工作 |
| EG-J01 | `gitguard` | 七项目新增工作 |
| EG-J02 | `gitguard` | 七项目新增工作 |
| EG-J03 | `gitguard` | 七项目新增工作 |
| EG-J04 | `gitguard` | 七项目新增工作 |
| EG-J05 | `gitguard` | 七项目新增工作 |
| EG-L01 | `flowguard` | 七项目新增工作 |
| EG-L02 | `flowguard` | 七项目新增工作 |
| EG-L03 | `flowguard` | 七项目新增工作 |
| EG-L04 | `flowguard` | 七项目新增工作 |
| EG-L05 | `flowguard` | 七项目新增工作 |
| EG-S01 | `specguard` | 七项目新增工作 |
| EG-S02 | `specguard` | 七项目新增工作 |
| EG-S03 | `specguard` | 七项目新增工作 |
| EG-S04 | `specguard` | 七项目新增工作 |
| EG-S05 | `specguard` | 七项目新增工作 |
| EG-S06 | `specguard` | 七项目新增工作 |
| EG-T01 | `testguard` | 七项目新增工作 |
| EG-T02 | `testguard` | 七项目新增工作 |
| EG-T03 | `testguard` | 七项目新增工作 |
| EG-T04 | `testguard` | 七项目新增工作 |
| EG-T05 | `testguard` | 七项目新增工作 |
| EG-T06 | `testguard` | 七项目新增工作 |
| EG-P01 | `guardcore` | 七项目新增工作 |
| EG-P02 | `specguard` | 七项目新增工作 |
| EG-P03 | `archguard` | 七项目新增工作 |
| EG-P04 | `codeguard` | 七项目新增工作 |
| EG-P05 | `testguard` | 七项目新增工作 |
| EG-P06 | `gitguard` | 七项目新增工作 |
| EG-P07 | `flowguard` | 七项目新增工作 |

## 插件任务的区别

EG-F / EG-H / EG-R / EG-G / EG-W 分别属于 codegraph-plugin / codeguard-plugin / codereview-plugin / gitflow-plugin / flowguard-plugin，保留现有 tasks 链接。它们是兼容宿主与证据集成任务，不等同于新产品实现任务。EG-J 和 EG-L 分别负责 GitGuard、FlowGuard 新产品及规则迁移，明确调用现有规则或接收规则所有权的时点。

EG-K12 是 FlowGuard 主责的五产品联合验收任务，各产品提供本域证据；EG-K28 是全系列收口，不能由负责人替其他产品签署未执行的验收。EG-K16 只编排规则变更/例外批准，规则是否允许例外仍由各专业域定义。EG-K23 的并行任务冲突由 GitGuard 负责，ArchGuard 提供符号/接口兼容事实。

## 旧条目的解释优先级

需求 ID 和任务 ID 保留便于追踪，不把前缀 K 解释为必须在 codeguard 实现。工程门禁的通用执行机制归 GuardCore，领域证据的判定规则归专业项目，阶段组合归 FlowGuard。既有详细任务涉及多域时由 owner 编排并通过接口依赖消费，禁止直接把其他域代码写入自身仓库。
