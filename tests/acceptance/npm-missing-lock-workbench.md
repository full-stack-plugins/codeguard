# npm缺锁文件的准备阻塞验收

初始化物理工作区、可绑定清单、package-lock.json明确NotFound时，在启动原生工具前保存观察0.2：lock_state=missing、lock_sha256=null、component_count=0、advisories=[]；authority仍local_unverified，coverage_proven=false。不安装、生成锁、执行包脚本或确认漏洞。

公开cve、显式选择npm的check all、task verify关联同构建根原稳定完整性任务。重复扫描不增任务，失败复检持久为incomplete；补齐锁后原收据过期，普通0.1观察继续复用任务。悬空链接、目录或不可读锁不等于不存在。错误字段与恢复锁后重放报告被拒，坏报告不新增任务。

新增npm_missing_lock_cli先RED确认未生成任务，再覆盖上述行为及五个坏/过期报告。51项普通集成回归通过。独立Draft202012验证实际缺锁观察/check/verify，五个伪造变体及旧消费者拒绝。当前独立与嵌入schema同步，新消费者check_feedback0.25/task_verification_preview0.5/check_aborted0.6；旧schema原字节归档。abort本轮只有既有单位用例及schema静态检查，未完成外部实际中断schema验收。

真实npm11.16.0/Node v24.18.0 check all回归通过（22.98秒）；这是局部原工具回归，不证明漏洞源覆盖、时效或交付许可。其余16项普通命令忽略的原生测试本轮未运行。

尚缺：坏清单/不可读文件准备任务、无显式npm选择的全项目自动任务、可信工具上下文、完整CVE覆盖、任务正式关闭及宿主门禁。不得将本验收视为OpenSpec全计划完成。

最终补充：2项check_command中断单元测试通过，共53项普通用例；Clippy、fmt及四份当前schema静态验证通过。观察0.1不得声称缺锁，必须使用0.2明确状态。

后续进展：可绑定坏清单的准备任务已由npm-invalid-manifest-workbench.md支持；不可读或缺清单/目录输入仍待独立准备协议。

后续进展：当前Unix check all默认发现并调度npm准备任务，不再要求显式npm参数才能入任务图，见npm-automatic-preparation.md；可信上下文自动解析和宿主Hook仍未实现。

后续进展：显式CVE/任务复检已支持npm-input-state-workbench.md所述不可用清单/锁状态；当前消费者check0.26/verify0.6/abort0.7。完整历史根恢复发现、外部工具/配置状态及宿主闭环仍缺。
