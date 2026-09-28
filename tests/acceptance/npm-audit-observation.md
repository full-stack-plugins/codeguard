# npm原生审计基础验收

Rust适配器新增npm11/auditReportVersion2机器报告观察与固定离线原生命令计划。依据本机既有npm11.16.0的Arborist AuditReport/Vuln及npm-audit-report退出契约核对组件、直接标记、原生advisory source、受影响范围、节点和间接via/effects关系。source不是CVE编号、range不是解析版本，观察不批准锁归属、数据库新鲜度或覆盖。

原始JSON最多8MiB，递归拒绝重复键；版本/退出/计数、路径、组件归属、引用、直接advisory严重度降级或报告error对象均未完成。依赖类别计数可重叠且包含项目根，不能将prod/dev等简单相加冒充原生依赖总数。间接组件保留via关系，不凭关联复制advisory编号。

命令计划使用显式Node/npm JS入口、用户和全局配置、私有cache；固定audit/json/package-lock-only/ignore-scripts/offline/audit-level=info，不Shell、不fix/安装或联网。执行层仍须冻结原项目cwd/清单/锁/npmrc、工具与完整运行时闭包；计划本身不授予配置或覆盖权威。

只读配置API识别package.json中明确npm audit脚本，返回configured/脚本未观察到/复杂调用待核对/损坏；复合Shell命令及其它参数不执行。缺脚本只能说明该清单没有观察到声明，不能证明外部CI或其它配置不存在。此API尚未接入detect/init/check all，不能声称项目CVE配置发现已完整。

缺解析API先RED；直接高危advisory而组件标低危的反例再次RED后修正。5项协议/命令/配置用例通过，覆盖原生退出1、版本/计数/路径/重复键、间接关系、字面路径及不执行脚本。另9项既有ESLint规则/公开CLI回归通过，共14项普通测试；CLI目标及adapter库Clippy -D warnings、fmt通过，最终记录见OpenSpec verification。

真实Node24.18.0/npm11.16.0经统一Rust runtime验证版本、空锁原生JSON与缺锁error两个审计场景，0.68秒通过：package.json及存在时锁文件字节不变，没有node_modules、项目脚本标记或安装。测试离线且缓存私有，不把协议里的合成advisory样本冒充真实数据库漏洞检出。

公开CVE/依赖命令、自动配置接线、真实非空依赖/漏洞正例、锁图与解析版本绑定、当前漏洞数据时效、批准策略、稳定任务/复检及宿主门禁仍未完成。7.3/7.5保持未勾选；本验收只是原生审计适配基础。

## 非空锁文件离线反例（2026-09-27）

实际 npm 11.16.0 在私有空缓存及离线模式下，对一个普通锁节点返回退出0、dependency total=1和零漏洞。报告一致并不证明漏洞数据库查询或覆盖完成。Rust观察固定 advisory_coverage=not_evaluated，即使锁节点关联成功也不授权CVE通过。原生测试覆盖空锁、非空锁及缺锁，核对清单/锁字节、无 node_modules 和脚本标记。尚缺真实漏洞正例、数据库新鲜度及完整任务接线。
