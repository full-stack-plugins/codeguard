# WASM 扩展回归发现的 ESLint 工作区边界与契约漂移

对应12.1/12.9/14.x，基于8fcf0aa实际运行三个基础crate的WASM全目标与CLI从Cargo metadata生成的201个集成目标（排除用户未提交erlang_native_differential），同时包含CLI lib/bins。基础层80组420通过/0失败/10条件忽略；CLI运行到eslint_lint_cli时终止，64组546通过/2失败/48忽略，退出101。该轮不代表完整WASM通过，后续目标没有执行。原始失败日志保留，不能加总成成功。

两个失败分别处理：

- 外部JS目标在WASM回退中返回泛化not_connected，缺明确边界诊断。现在source.strip_prefix失败时显式source_outside_workspace，语法不运行、task_id为空，已初始化工作区不新增finding。既有测试先RED后GREEN。
- 缺全部上下文的合法JS文件按既有候选契约可推荐原生lint，不强制制造环境阻塞。旧准备测试却要求new_blockers=1。将其明确改为部分显式原生上下文（只给missing-node），确保不启用WASM回退，继续覆盖稳定环境任务、重复扫描、原生恢复和零finding。产品的初检通过后推荐安装契约不改，独立javascript_lint_candidate仍验证无上下文候选与任务行为。

修正后WASM两目标12通过/0失败/4条件忽略；追加实际报告采集后eslint目标6通过/4忽略（重叠不累计）。默认ESLint6通过/4忽略。外部目标实际0.6反馈通过原schema，没有修改旧协议。WASM CLI全目标严格Clippy、分层、OpenSpec strict、diff通过；受保护Erlang草稿摘要不变，没有执行。

完整CLI WASM回归仍需重新执行，不能用上述定向GREEN代替。独立holdout/真实原生全矩阵、六项语料差异、宿主、跨平台和CI证据源前置仍开放，父任务不勾选。8fcf0aa CI37426760943已终态：MSRV通过，gate失败于Check out corpus evidence source。日志摘要见evidence/wasm-eslint-boundary-regression-2026-10-06.json。
