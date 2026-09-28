# check all逐npm构建根编排

Unix `check all WORKSPACE --node-tool ABS --npm-entry ABS --npm-version VERSION --userconfig ABS --globalconfig ABS [--registry URL]` 使用已发现的package.json构建根。显式npm上下文进入共享任务图，每根保持独立cwd、临时运行区和任务；所有任务共用请求截止时间及jobs上限。没有显式上下文时保持配置发现，不擅自安装工具或继承凭据；check java拒绝npm参数。

执行阶段返回原生反馈与待同步观察，汇总阶段串行同步，避免并发争抢工作台锁。已初始化父工作区中自动生成稳定CVE完整性任务及next指引，子目录不另建任务工作台。发现阶段清单摘要在扫描前后核对；坏上下文、原生错误、超时与覆盖未核验均不能成为源码违规或交付通过。

普通二进制验收覆盖两根并发、重复扫描各一张任务、一个根失败而另一根结果保留、共享500ms截止时间停止余下任务、重复参数和局部语言范围限制、缺显式上下文不自动执行。原子序列的同刻度64个ID用例验证并发运行身份唯一；旧进程/时间身份无法保证这一点，早期并发回归曾出现私有目录创建失败，未把该失败归为源码问题。

check_feedback升级0.24，新增npm_cve逐根结果与候选原因；check_aborted升级0.5保留已取得npm结果。旧0.23/0.4协议另存版本schema，旧消费者不得解释新版本为普通allow。当前版本与无上下文实际反馈schema校验、三项伪造权威/覆盖/allow反例及旧0.23形状校验通过。

真实既有Node24.18.0/npm11.16.0的空锁离线check all → 持久任务 → next通过（初次21.80秒）；清单和锁不变，无node_modules、安装或包脚本执行。该用例不证明可信漏洞库覆盖、完整原生模块/配置闭包、正式任务关闭或宿主门禁。

仍待完成自动解析可信工具/审计源上下文、早期准备失败全部持久化、完整依赖图与漏洞库时效、全部语言/category调度、正式关闭/重开、跨平台及完整交付验收。

最终验证：66项不同普通用例通过；真实npm check all最终22.30秒通过。其它9项显式原生用例本轮未运行。Clippy（-D warnings）、fmt、OpenSpec严格校验及diff检查通过。

后续进展：当前Unix check all默认发现并调度npm准备任务，不再要求显式npm参数才能入任务图，见npm-automatic-preparation.md；可信上下文自动解析和宿主Hook仍未实现。
