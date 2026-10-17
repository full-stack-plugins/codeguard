# 同字节原生零诊断的grammar反证候选

## 问题与实现

首次WASM对合法Kotlin `object C { val value = 1 }` 返回不可定位恢复未完成。后续原生零诊断原来仅给“核对反证或修复归因”的笼统提示，智能体无法区分原始源码合法和改代码后的复检。

`syntax_task_recheck`现在只在历史报告已核对、当前输入稳定、原生结果完整且无语法诊断、首次grammar摘要有效、原生目标与首次源码摘要完全相同时，给出“同一源码的grammar反证候选”指引。源码变化、过期观察和没有grammar引用的原生首次来源不套用这段描述。它引导调查grammar版本/兼容或扫描预算，保留原字节与原生证据，不重复源码修补或安装工具。

现有版本、字段、任务身份及关闭权限保持不变。观察仍为local_unverified/candidate_absent_unverified_policy，不能自动添加白名单、批准门禁或关闭任务。工具锁、可信项目策略和完整覆盖尚未核验；单文件原生零诊断不是完整lint通过。

## 实际RED/GREEN及原生证据

新增受控同字节测试实际RED：next仍返回笼统指引。修复后Kotlin任务目标4通过/0失败；测试同时覆盖当前源码未修改、源码变化后的旧证据失效及不同源码的再次复检不误称同字节反证。

本机已经安装的kotlinc-jvm2.4.10（JRE26.0.1）显式原生测试实际1通过/0失败/0忽略，7.47秒；源码摘要与首次WASM相同，原生没有语法或上下文诊断，源码不变，任务保持open。工具身份仅launcher_only，不冒充compiler JAR/JDK完整批准。首轮真实测试用不支持的`--timeout=90s`参数被CLI拒绝；使用已有受支持的`--timeout 90s`后通过，未放宽参数契约。

[完整实际复检及next报告](evidence/kotlin-same-source-counterevidence-2026-10-06.json)。当前目标source_sha256及original_report.source_sha256同为`9a40dcd89c137534d57763addbb8d45368fed76bd046c5198c4fd3a016e6414a`；grammar摘要`c80c88867a589a1a0959bcea89de84b7e9684b3693b2cdb2944812458e62ff48`。

六个受影响WASM目标的回归已实际完成，精确统计及静态终态下方追加。真实目标单独执行，不与普通/重叠结果累加；默认忽略的外部工具项不计通过。

本批不更换grammar字节、不修复原始358例回放中的unknown/FN、不提高32语言资格，不改变真实宿主与发行状态。插件源提交PR分支授权和grammar生成器授权仍未收到；CI的缺固定插件源阻塞不因本批本地测试消失。

## 最终核验

六个受影响WASM目标实际49通过/0失败/2条件忽略；追加归档身份契约后的Kotlin目标5通过/0失败/1条件忽略，与上述重叠，不累加。真实kotlinc目标另行显式1通过。默认及WASM两种CLI全目标严格Clippy均通过；所改Rust文件格式、OpenSpec strict、crate分层及diff检查通过。未执行完整workspace或358例重放，用户Erlang草稿摘要保持不变。

97dfba7远端CI37397096417已终止失败，日志明确为固定插件提交dec5f9d不在远端可达；尚未运行的后续CI任务不计通过。本批不绕过源审计、不替换固定来源。
