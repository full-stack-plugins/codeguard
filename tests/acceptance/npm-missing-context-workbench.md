# npm缺少原生上下文的持久修复任务

范围：已有可读清单及锁文件的初始化物理工作区。公开cve缺参数、显式选择npm的check all部分参数、task verify缺参数，保存npm_execution_context_missing本地观察；没有原生执行、组件或advisory，也没有交付授权。

缺参数重复扫描与补齐后的原生扫描沿用每构建根同一稳定完整性任务。未初始化不创建工作台；取消、过期和输入不符不产生当前可用证据。task verify失败保留incomplete事件，任务不关闭。准备观察不得夹带非空advisory，原始未验证权限与coverage=false不变。

验收：npm_audit_cli原任务无持久记录、准备记录夹带真实格式advisory两个RED反例先失败，再通过。check_all_npm验证两个子根各一稳定任务、部分上下文不启动受控工具、随后完整上下文执行仍沿用任务；npm_workspace_scope_cli验证父工作区归属及严格复检收据。

36项受影响普通测试通过；真实反馈、保存报告和失败复检以独立Draft202012验证，四个伪造权限/覆盖/组件/advisory变体拒绝。CLI与本次受影响测试Clippy通过。受控脚本为协议夹具；真实npm原生回归单独记录，不能据此宣称漏洞数据覆盖有效。

尚缺：缺锁或坏清单时的独立输入状态协议；未显式选择npm的check all自动准备任务；宿主可信工具上下文、数据源完整性/时效、完整覆盖及正式交付关闭。现有观察schema 0.1的diagnostic_reason允许准备原因，本次不新增字段；所有当前嵌入消费者同步零组件/空advisory约束，旧版归档不改。

真实原生回归：既有Node v24.18.0/npm11.16.0公开CVE用例通过（44.13秒），不安装或执行包脚本。实际check all准备反馈schema通过，两个当前嵌入观察schema与独立schema一致；fmt检查通过。其余2项忽略原生测试本轮未执行。

后续进展：缺锁文件已由npm-missing-lock-workbench.md所述观察0.2支持；坏清单/不可读文件准备任务仍缺。check/verify/abort当前消费者版本分别0.25/0.5/0.6，本文早期普通0.1观察继续读取。

后续进展：可绑定坏清单的准备任务已由npm-invalid-manifest-workbench.md支持；不可读或缺清单/目录输入仍待独立准备协议。

后续进展：当前Unix check all默认发现并调度npm准备任务，不再要求显式npm参数才能入任务图，见npm-automatic-preparation.md；可信上下文自动解析和宿主Hook仍未实现。

后续进展：显式CVE/任务复检已支持npm-input-state-workbench.md所述不可用清单/锁状态；当前消费者check0.26/verify0.6/abort0.7。完整历史根恢复发现、外部工具/配置状态及宿主闭环仍缺。
