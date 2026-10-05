# 当前构建只读命令支持帮助

规格：`introduce-rust-codeguard-cli` unified-cli-contract新增Help要求，2.1/2.7/12.9继续跟踪。

## 问题和实现

旧`--help`是一段手写长文本，仍含过期check_feedback0.34和插件未接线描述，不能按命令查询或返回JSON；新增四测试首轮3失败/1通过，确认目标能力缺失。

现在C01–C36及8个额外公开操作来自同一静态描述目录（44行）。`help`/`--help`/`-h`支持完整目录、精确前缀查询和human/json；如`help task verify --format json`。`check --help`、`task verify -h`等精确前缀末尾帮助也直接返回静态视图；路径/工具参数不会被吞掉假装帮助成功，完整语言专用帮助和全部参数矩阵仍待实现。

支持状态implemented/partial/planned/unavailable_build与executable分别明确；executable只说明入口存在，不代表原生已配置、工具安装成功、完整验收或交付许可。独立security/dependencies/fix/mcp/compat和两个完整gate仍标planned；tools install仅未批准预览，apply阻塞；实际lint只列八个直连语种，Ruby不被错误宣称为已接入。私有worker不列入公开目录。示例包含Go/Erlang/Swift/Kotlin/Zig/Ruff等真实选项前缀，但绝对路径必须替换且仍由命令校验，不构成自动执行或安装授权。

帮助只依赖编译目录/版本和特性，不读取项目或PATH，不执行目标入口。查询成功退出0，报告native_execution=not_run、delivery_decision=not_evaluated；未知命令、重复格式、SARIF和多余路径退出2且stdout为空。旧CLI检查/查询分发保持原逻辑。

## 实际证据

默认及WASM两构建各运行13项目标回归（帮助5、规则目录6、版本2），0失败/0忽略。只读临时工作区中的AGENTS字节和唯一文件保持不变；未实现mcp查询不启动服务。两份[默认](evidence/command-help-2026-10-05-default.json)/[WASM](evidence/command-help-2026-10-05-wasm.json)真实JSON报告按0.1封闭schema校验，grammar probe分别unavailable_build与partial。开发schema2项通过，拒绝未知major、伪执行、伪许可、自批字段和planned可执行标志。

## 未完成

静态目录不是完整命令注册/参数生成器，不能替代逐入口全部参数、默认值、别名、错误/取消/恢复矩阵或Rust统一MCP服务。help语言/路径上下文的完整形式仍缺。当前源码不等于公开npm0.1.4已包含新帮助。2.1/2.7/12.9父任务不勾选，完整目标保持。最终静态及回归结果后续追加。

## 最终本地验证

2026-10-05：`CARGO_PROFILE_TEST_DEBUG=0 cargo test --workspace --locked` 实际结束退出0，249个目标结果累计1368 passed、0 failed、123 ignored。默认构建不覆盖WASM专用测试，忽略项不计入原生工具或发布验收。此前默认/WASM定向各13项通过，两个构建的`cargo clippy --workspace --all-targets -- -D warnings`分别9.64秒/10.82秒通过；实际报告schema2项、改动Rust文件格式、分层、OpenSpec strict及diff检查通过。

CI增加WASM构建的help/rules/version定向回归，以免默认suite遗漏特性差异。远端CI结果须按本次提交单独核对，不借用旧提交成功。用户已有Erlang差分草稿未修改、未纳入本批提交，也未作为原生验收依据。
