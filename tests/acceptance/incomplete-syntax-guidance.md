# 未完成解析的源码修复分流

固定Kotlin/Swift grammar的隐藏错误与扫描预算耗尽都可能生成syntax_recovery_incomplete；该状态本身不证明源码违规。原项目人类输出却一律说grammar报告错误，任务和对话没有明确区分原生合法/有诊断后的下一步。

实际三个RED：项目终端、持久任务正文、Claude上下文分别缺少原生确认合法后的grammar/预算排查动作。现在项目人类输出使用中性未完成描述；候选JSON下一步、Claude对话、任务正文及next/task show统一要求对原始源码运行适用原生工具。原生确认合法时调查grammar版本/兼容性或扫描预算；诊断成立时才按真实位置修复。原报告字段、版本、枚举、任务身份及关闭权威不变，位置仍不虚构。

已实际验证：项目隐藏Kotlin目标1通过/0失败/0忽略；17项Hook语法任务目标通过；新鲜和过期任务的step增加合法分支断言，缺工具或过期原生历史的step覆盖额外复现RED：缺失新分流；已在syntax_task_recheck补齐后，WASM五目标56通过/0失败/3条件忽略，默认四目标39通过/0失败/3条件忽略。项目终端/JSON目标额外1通过。旧历史与普通任务身份保持兼容。本批不修改grammar、不裁定Kotlin/Swift未知样例、不提高32语言资格或发布版本。

实际日志：`/private/tmp/codeguard-incomplete-guidance-{project,task,hook}-red.log`及`project-green`、`hook-green`、`default-regression`、`wasm-regression`。三个RED各一失败；忽略项不计通过。原提交973672c之前的1516通过默认全suite不是本次修改的完整回归证明。

远端973672c CI37396179981失败于固定插件源码dec5f9d未发布；本地插件main比远端f09c074多三个提交。直接push实际被GH013拒绝，要求PR及三项检查，新的分支/草稿PR授权已询问且尚未获得。CI增加构建前固定对象可达检查，缺来源不能跳过审计或换用旧源码；该外部交付阻塞独立于本次反馈测试结果。

最终默认/WASM CLI全目标严格Clippy、定向rustfmt、OpenSpec strict、crate分层和diff检查通过。默认四目标在追加历史分流修复后再次39通过/0失败/3条件忽略，WASM五目标56通过/0失败/3条件忽略；两构建/重叠目标不相加。未执行完整默认/WASMworkspace或358例回放；用户Erlang草稿字节保持不变。
