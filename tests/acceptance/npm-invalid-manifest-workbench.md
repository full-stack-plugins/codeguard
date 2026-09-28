# npm坏清单准备阻塞

目标：原生启动前处理JSON语法、重复字段和scripts类型错误，避免无效清单被反馈成版本失败，更不能成为源码漏洞。原工作区已初始化、清单与锁可绑定时，保存npm_manifest_invalid/npm_scripts_invalid观察；零组件与空advisory，权限仍未验证，不产生CVE覆盖或任务关闭。

npm_invalid_manifest_cli先RED证实原工具仍启动且反馈版本失败，再覆盖完整参数与check all部分参数均不启动受控工具、稳定任务、task verify失败历史、明确清单修复步骤、输入恢复后旧收据过期，以及伪造当前摘要但不实声明清单坏被拒。输入变化会使准备证据失效；工具输入冻结时重新比对初始清单摘要。

验收：35项不同普通回归通过；独立Draft202012验证实际观察/check/verify，3个伪造组件/advisory/覆盖变体拒绝。真实npm11.16.0/Node v24.18.0 check all回归通过（22.50秒），不安装或执行包脚本。另10项忽略原生用例本轮未运行。

相邻缺锁协议0.2仍明确missing及空摘要；坏清单同时缺锁时保留缺锁阻塞和配置错误反馈，不虚构锁。普通坏清单观察0.1结构字段不变，当前嵌入消费者同步零组件/空advisory条件，历史schema不改。

尚缺：不可读/缺清单/目录输入持久准备状态、可信工具上下文、全CVE覆盖、正式关闭与宿主门禁。本验收不代表OpenSpec整体完成。

最终检查：Clippy -D warnings、fmt和OpenSpec严格验证通过。

后续进展：当前Unix check all默认发现并调度npm准备任务，不再要求显式npm参数才能入任务图，见npm-automatic-preparation.md；可信上下文自动解析和宿主Hook仍未实现。

后续进展：显式CVE/任务复检已支持npm-input-state-workbench.md所述不可用清单/锁状态；当前消费者check0.26/verify0.6/abort0.7。完整历史根恢复发现、外部工具/配置状态及宿主闭环仍缺。
