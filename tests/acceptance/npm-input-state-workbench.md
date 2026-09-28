# npm不可用输入与持久任务

新增观察0.3：manifest_state及lock_state各为present/missing/not_regular/path_alias/unavailable；只有present有SHA-256，其余null。诊断按首个不可用输入推导，零组件/空advisory、未验证权威、无覆盖及交付许可。旧0.1普通与0.2缺锁严格保留；消费者check0.26/verify0.6/abort0.7，历史schema原字节归档。

npm_input_state_cli先RED证明缺失误表达为调用上下文缺失，后续RED证明目录使next因attempt_input_unreadable失效。输入状态观察不读取链接目标、不打开非普通文件；任务及失败复检跨missing/目录/链接/超限复用，恢复后历史过期。显式完整参数也不启动尚未准备好的原生工具。尝试指纹只在可读时记录源码摘要，其它状态供环境修复/no-progress历史使用。6种伪造/错误身份与1条过期报告拒绝，不增任务。

78项不同普通回归（76集成+2中断单元）通过；真实npm11.16.0/Node v24.18.0 check all回归通过（22.42秒），不安装或执行包脚本。实际观察/check/verify通过独立Draft202012，6个伪造变体拒绝，旧消费者拒绝新协议。其余16项忽略原生用例本轮未执行。unavailable实际fixture为超限；独立EACCES权限失败运行未验收。abort仅单位和schema静态检查，非外部真实中断协议验收。

边界：显式CVE/既有任务可描述丢失清单；默认check all的完整历史根恢复发现仍缺。外部工具/配置输入持久状态、Windows、可信上下文/漏洞覆盖、宿主Hook、正式关闭与门禁均未完成。

最终核验：Clippy -D warnings、fmt、OpenSpec严格验证通过；最新明确参数与修复指引回归实际执行通过。

后续有限恢复：npm-historical-roots-workbench.md已验收当前0.3初始化画像中清单不可用、物理目录仍存在的准备范围恢复；整体目录消失及画像刷新删除旧根后的完整义务恢复仍缺。
