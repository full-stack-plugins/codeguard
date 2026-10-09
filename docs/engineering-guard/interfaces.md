# 七项目统一接入契约

七个产品最终均支持 CLI、MCP、GitHub Actions、Agent Hook 和 API。这是目标能力矩阵，不是现有支持声明；逐项目逐入口验收，统一协议不等于所有入口同时上线。

## 1. 接入面

| 面 | 拟议接口 | 实现与权限边界 |
|---|---|---|
| CLI | `<product> doctor/plan/check/evidence`；GuardCore 提供 contract/plan/run/evidence/policy | 六守卫 check 检查本域，GuardCore run 执行已批准 provider 计划；不提供虚假的通用质量 check |
| MCP | 按产品命名空间的能力查询、计划、检查、证据读取 | rmcp；取消与副作用声明；不提供通用 shell、批准签发或无限范围 Git 写工具 |
| GitHub Actions | 每项目版本化 Action 描述或复用工作流，统一输入/输出 envelope | 固定 action/制品/策略摘要，独立控制端验证生产者及候选；required check 配置另行验收 |
| Agent Hook | 每项目声明事件映射与适配入口，共享去重机制 | 优先复用现有插件安装入口；Hook 可绕过，不能自称远端安全边界 |
| API | Rust library API + 版本化 JSON/HTTP 服务适配，发布 OpenAPI 契约 | 共享 DTO/授权/幂等；默认本地或私有部署，不因支持 API 自动公开监听 |

路径、命令、HTTP endpoint 和 MCP tool 名称均为拟议，由 EG-P01—07 冻结；GuardCore 机制 API 不承载各专业域算法。GitGuard 写操作与 FlowGuard advance 分开暴露，必须显式动作授权；常规验证不顺带推进阶段或合入。

## 2. HTTP 与长任务

拟议长任务模型 `POST /v1/runs` 返回 operationId，`GET /v1/runs/{id}` 读取状态，取消为独立动作；重复幂等键只能对应相同输入摘要，不同输入拒绝复用。请求和响应具有 schemaVersion、product、action、subject、contractDigest、operationId；大报告通过鉴权 artifact 引用获取。

API 认证身份来自连接层和可信 identity provider，不能相信 body 中的 user/tenant/trusted；校验 audience、repo/task/tenant scope、撤销/期限及证据读取权限。输入上限、并发/超时、日志脱敏、取消、断连后的状态对账必须验收。签发接口只在受信控制面部署，Agent 可访问的 API/MCP 不自动获得签发权。

## 3. 阶段和资格

第一阶段先完成五个产品的 CLI + 最小 CI/Hook，核心协议稳定后接 MCP/API。第二阶段 SpecGuard/TestGuard 加入相同协议。所有产品的五种入口均纳入 EG-P01—07 完成门槛；某入口 pending/unsupported 时只声明已验收的范围，不用一个通用 wrapper 的测试代表七个产品。

资格矩阵维度：product × interface × version × host/platform × mode；必须记录真实安装、正常/拒绝/未知、取消、重入、精确候选、授权拒绝和升级回滚。跨入口等价是同一输入产生同一规范化领域结果，传输特有字段和错误另外校验。
