# C/C++确认编辑文档Hook局部验收

对应introduce-rust-codeguard-cli/hook-protocol的9.9/11.17/15.3/15.6。新增缺上下文测试先因Null反馈失败；复用check_c_family_comments后通过。hook execute允许显式绝对Clang与c11/c++17编辑档案，共享事件截止时间、已校验的最多8个所选文件及原观察预算。缺工具/标准为context_required，不猜项目编译配置、不生成源码违规。原生警告与结构规则分开；native_unwired_files仍保留C/C++语法缺口。初检恢复节点需要原生确认的优先级不能被文档反馈覆盖。

反馈hook_fast_feedback0.18、外层0.31，包含selected_files文档包装与c/cpp观察；不改变旧报告。失败写入的工具/标准配置不消费，禁止相对工具路径、重复配置、不支持标准。repair_ready仍通过原任务恢复标准，不能使用编辑档案覆盖。无工作区不自动初始化；已有工作区连接警告和结构任务，重复编辑两类ID稳定。init部分就绪退出3在测试中保留，不冒充质量策略就绪。

默认/WASM四目标共各40通过、0忽略，实际Clang和Ruff均显式执行；追加稳定任务断言后编辑目标两模式各2通过。相邻C-family repair_ready默认原生目标通过。首次相邻全包含调用漏传Ruff变量失败，补齐变量后重跑通过。测试初始化首次错误假设退出0，校正为真实部分就绪契约后通过；未修改产品退出语义。

4份实际编辑报告与测试/CLI摘要及稳定任务ID在evidence/c-family-edit-{default,wasm}-{c,cpp}.json，生成在整轮测试断言完成之后。报告校验包括语言/标准交叉混配、资格/交付伪造；evidence/c-family-edit-schema.json保留数量。完整源码注释契约准确性、头文件、编译数据库、宏/项目上下文、原生语法完整路由、已安装宿主、自动配置发现、可信关闭/复发、跨平台/精度和发行仍未验收。66完成/288待完成、228核心blocked与0/32资格不变。
