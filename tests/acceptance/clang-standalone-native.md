# 明确标准的 Clang 独立原生入口

对应 `introduce-rust-codeguard-cli` 8.26/8.29、14.5/14.6。固定 Apple Clang 21.0.0 (clang-2100.3.34.2)，仅明确 `--clang-tool ABS --standard c11|c++17` 的同语言单文件请求激活原生档案。不代替clang-tidy、注释、安全、依赖检查，也不从请求标准推断项目编译模型。工具和标准不配对、错语言/标准或重复参数在执行前拒绝。

依据：[Clang参数参考](https://clang.llvm.org/docs/ClangCommandLineReference.html)、[Clang用户手册](https://clang.llvm.org/docs/UsersManual.html)。使用runtime固定argv：禁止默认配置，fsyntax-only、stdin、明确语言/标准、nostdinc、有界SARIF、禁颜色/caret，并明确启用Wall/Extra/Pedantic档案。不执行源码，不生成对象文件。继承环境清空、cwd为独占私有目录；所有进程共用deadline。冻结并复核工具字节及别名，复核源码，变化不保留旧诊断。

预处理内容保守保留 `clang_preprocessor_context_unresolved`；没有编译数据库和include/宏前置时不将其转为源码违规。该判断可能对字符串中的井号也要求确认，是明确的覆盖限制，不是源码违规。其它标准、Clang版本、平台及动态上下文未验收，不隐式安装或调用项目构建脚本。

SARIF固定同stdin的file://制品、driver版本、Unicode码点列、规则索引及执行成功语义，逐结果和全部位置核验后仅保留规则ID与UTF-8字节行列。自由诊断文本不进入反馈；外部文件、坏列/版本/索引、重复JSON键、后续坏结果和退出/空诊断矛盾拒绝。选定原生失败不回退WASM，零诊断不签发完整lint或交付通过。

公开参数缺失反例先RED；接通后发现human仍声称未接入且没有原生位置，独立RED修复为原生状态/规则/位置反馈。严格Clippy指出测试使用PathBuf引用，修正为Path后重跑；没有关闭lint规则。

默认两个目标7 passed/0 failed/1 ignored；WASM四目标21 passed/0 failed/1 ignored。解析器契约1 passed，含Unicode及多个报告矛盾反例。真实已安装编译器独立目标1 passed/0 failed/0 ignored：C11和C++17分别运行非法ASCII、非法Unicode、warning及修复后的源码，八份真实反馈中原生错误/字节位置与零诊断对应，源码不变。该真实目标与默认fixture证据分开，不把忽略计作成功。补充复检argv后的默认及真实目标重新执行。

报告为 syntax_lint_feedback 0.2；明确路径、冻结源码摘要、原生工具/版本、限定标准及可复用验证argv，保留project_configuration=unknown和交付未评估。Clang原生观察的工作台导入、项目/编辑原生调度、task verify、可信关闭与重开、实际宿主及发行尚未接线；syntax_tasks及setup.task_id保持null，父任务不勾选。证据：evidence/clang-standalone-native-2026-10-06.json。受保护Erlang草稿未执行或提交，32grammar资格仍0。

warning退出0仍有原生诊断的公开反例先RED；最终启用Wall/Extra/Pedantic及等级化读者，保留warning而不当作无诊断。真实档案八份反馈与受控四份反馈分开；旧语法差分语料不因为警告档案升级改变oracle。

最终封闭协议校验：受控四份和真实八份报告共十二份通过0.2 schema，八种矛盾变体被拒绝；368份schema元定义通过。默认及WASM的CLI/adapters全目标Clippy -D warnings均终态通过；crate分层、OpenSpec strict及diff检查通过。以上只验收明确标准的独立原生入口，项目及工作台父任务继续开放。
