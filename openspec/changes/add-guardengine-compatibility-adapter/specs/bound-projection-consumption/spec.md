## ADDED Requirements

### Requirement: Approved references require authenticated consumption
消费层 SHALL 通过受信控制器核验批准基线不可变digest/ref及审批身份、范围、有效期和撤销。摘要重算 MUST NOT 当作来源认证；适配器 MUST NOT 签发批准或以工作区boolean自证信任。

#### Scenario: Approval is forged locally
- **WHEN** 候选文件写accepted或提交自建issuer
- **THEN** 拒绝用其满足批准要求，不执行授权动作

#### Scenario: Approval service is unavailable
- **WHEN** 消费时无法核验必需批准来源
- **THEN** 资格不成立，记录验证故障，不改成已批准或缺审批review的成功查询

### Requirement: Changed bindings invalidate eligibility
资格 SHALL 绑定repo/task/worktree/requirement、candidate/base/merge-group、源码/基线/contract、工具配置及mapping/analyzer/coverage版本。相关输入改变或批准过期/撤销 MUST 使结果不可复用；原始报告保持不可变。

#### Scenario: Analyzer or baseline changes
- **WHEN** 候选未变但mapping、coverage或基线digest改变
- **THEN** 旧资格失效并重算，不仅比较HEAD

#### Scenario: Approval is revoked after evaluation
- **WHEN** 技术报告可重算但批准撤销
- **THEN** 保留技术报告，消费资格失效，不能用缓存批准授权

### Requirement: Parallel projections preserve immutable ownership
并行投影 SHALL 隔离每个需求/task/worktree的义务及运行；重试有新runId，同次重复导入幂等。晚到结果 MUST 仅归档原绑定，不覆盖当前候选结果或另一任务记录。

#### Scenario: Two requirements share extraction
- **WHEN** 两个需求共享相同源码但有不同义务
- **THEN** 可复用匹配摘要工件，分别评估义务及批准，不交叉满足

#### Scenario: Old success arrives late
- **WHEN** head2结果已发布而head1 ALLOW迟到
- **THEN** 条件更新拒绝覆盖head2，head1仅保留历史

### Requirement: Protected gates consume exact queue candidates
可信CI消费 SHALL 针对GG-CANDIDATE提供的精确合并队列候选及base/group，重新取得native证据并投影。PR-head结果或多分支报告拼接 MUST NOT 满足该义务；技术ALLOW不授予合并/发布权。

#### Scenario: Queue candidate differs from PR head
- **WHEN** 队列合成候选M与已检PR head不同
- **THEN** 要求M的native检查/投影，旧结果只能历史参考

#### Scenario: Queue is regrouped
- **WHEN** 消费前base推进或mergeGroupId改变
- **THEN** 受影响资格失效并重新检查，不自动写refs

### Requirement: Projection storage is bounded and read-only by default
适配器 SHALL 默认不启动native工具、不联网安装、不修改源码或.codeguard任务。显式工件导出 MUST 限路径/链接/输入预算并原子发布，保留脱敏摘要审计和访问控制；不运行不可信policy脚本。

#### Scenario: Input contains a malicious path
- **WHEN** 报告引用越界链接或超预算工件
- **THEN** 拒绝读取/发布，无授权根外写入且无假完成

#### Scenario: Projection is retried without output request
- **WHEN** 重复读取同一既有报告且未指定导出
- **THEN** 无新增工作台事件、下载或子进程，输出不泄露env或秘密

### Requirement: Rollout preserves native ownership and rollback
发布 SHALL 经shadow、显式opt-in、已声明版本/平台/宿主矩阵验证及独立回滚；生产集成等待GE各适用门。native缺口由旧change拥有，未满足coverage MUST 阻断资格，不以回滚适配器绕过required checks。

#### Scenario: Selected native slice is unfinished
- **WHEN** 旧2.4或5.4等所需切片尚无完整证明
- **THEN** 仍可交付partial/unsupported adapter能力，不关闭旧任务或重造native实现

#### Scenario: Adapter is rolled back
- **WHEN** 新版profile与消费者不兼容而停用适配
- **THEN** native行为与历史报告仍可用，未满足集成门禁不改成通过；未知N/N-1拒绝消费
