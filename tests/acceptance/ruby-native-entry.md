# Ruby原生语法检查与稳定任务：局部链路验收

对应OpenSpec introduce-rust-codeguard-cli的native-tool-adapters Ruby场景以及S05/S08/S09/S14；全语言或修复工作流父任务保持未勾选。

## 行为与边界

`codeguard lint ruby FILE.rb [--ruby-tool /absolute/path/to/ruby] --format=json`采用显式或绝对PATH原生入口优先。固定Ruby2.6.10p210，关闭gems，向`--disable=gems -EUTF-8:UTF-8 -W0 -c -`提供冻结UTF-8 stdin，不执行用户代码、项目插件或自动安装。显式工具故障、新版本未验收、输出非法均保留incomplete，不切到WASM。缺原生入口时WASM构建提供候选初检；候选/不完整要求准备原生工具，完整零候选推荐准备。默认构建明确wasm_feature_not_built。

初始化工作区中，原生首次诊断/环境失败与WASM候选复用同一工作区、文件、语言指纹。重复扫描更新证据，任务清单不决定门禁。新零候选不创建源码任务；原生零诊断更新观察但不关闭现有任务。next提供当前原生行号、unknown列单位和原工具argv；task verify保存历史、失败尝试与原工具复检。缺/坏工具或输入变化后不沿用旧修复位置。错误语言工具、相对路径和重复工具参数在原生启动前拒绝。

项目Ruby版本兼容性仍unverified。必须核对项目声明版本是否适用Ruby2.6.10p210，不把新版本语法改成旧版本来绕过检查。原生诊断仅具行号，不猜测列号；RuboCop、注释、安全和完整项目检查仍未完成。报告不签发交付许可；可信关闭、原生聚合检查、Ruby3和公开发行仍开放。

## TDD与执行证据

初始公开入口0通过/4失败/1忽略，失败原因是没有Ruby入口；随后稳定任务测试失败。接线原生发现/复检后通过，再新增WASM候选任务红测，复现未创建任务。复用worker观察时发现group_id属于传输协议，历史候选保存明确的八项位置字段，经既有严格校验后持久化，不重复解析。

默认全工作区251个结果目标合计1387通过/0失败/125忽略，日志`/private/tmp/codeguard-ruby-workspace-default.log`。随后只增加两项工具错参/失败复检测试，最终默认Ruby目标11通过/0失败/1忽略，WASM Ruby目标12通过/0失败/1忽略。共享任务WASM回归11目标87通过/0失败/7忽略，覆盖Erlang/Swift/Kotlin/Zig/Hook和帮助；未执行完整WASM workspace，不使用用户未提交的Erlang差分草稿作为验收oracle。默认和WASM全workspace all-targets Clippy通过；增加最后两项测试后WASM all-targets Clippy再次确认。

本机已有/usr/bin/ruby2.6.10p210独立实测修复目标1通过。另一次真实CLI流程从WASM候选到同一原生任务，再复检still_blocked；修复后candidate_absent_unverified_policy，finding.state仍open。真实输出见[evidence](evidence/ruby-task-workflow-2026-10-05-native.json)、[复检](evidence/ruby-task-workflow-2026-10-05-verify.json)、[next](evidence/ruby-task-workflow-2026-10-05-next.json)。schema回归5通过，历史help0.1回归2通过，Rust反馈回归3通过/1真实Rust证据测试未执行；本批不借用历史真实Rust结果计通过。

## 协议与交付

帮助0.3、Ruby初始化工作区反馈0.3、原生首次历史0.9、语法复检0.10、任务预览0.23和修复简报0.15新增封闭schema；未初始化的入口保留Ruby反馈0.1。旧help0.1/0.2及所有旧报告/schema保持原件，禁止伪造列号、批准、coverage_proven和allow。OpenSpec strict、分层、fmt和diff检查通过。CI增加WASM Ruby目标，但远端CI与本机证据分开确认；公开npm0.1.4未变化。
