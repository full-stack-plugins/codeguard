# 当前九语言实际原生差分及历史报告保留

日期2026-10-06，对应12.3/12.11/14.17/14.19，基于dd4e992产品源码。只执行本机已存在工具：Apple clang21、Go/gofmt1.23.4、javac21.0.12.1、Node24.18.0、kotlinc2.4.10（JDK21）、Ruby2.6.10p210、rustfmt1.9.0-stable、Apple Swift6.4、Zig0.16.0。无安装、升级或下载；Erlang用户草稿未执行。

九个测试目标显式--ignored执行，共12通过/0失败/0忽略，退出0。工具10个入口（Go含gofmt）的规范路径/字节前后一致，此为入口核验，不代表完整分发bundle、动态库或受保护工具锁验收。Kotlin两项/Swift一项隐藏恢复仍为unknown。Go20例中1项逻辑行位置原生未知，其余19比较；组合7TP/12TN/0FP/0FN，raw仍2FN。Ruby18例5TP/13TN/0FP/0FN。Java21八个局部新语法例继续保留原生观察，不进入独立holdout。

JavaScript18例原始和当前组合均5TP/11TN/0FP/2FN：module_return与duplicate_binding。测试通过证明差分执行和当前期望契约，不是无漏检。源码审计发现差分入口统一run_syntax_worker_candidate，未像项目/组合回放使用run_syntax_worker_binding_candidate，重复绑定规则未被评测；下一步须对齐组合测量，原始恢复指标仍保留。module_return属于显式module原生上下文，不能对CommonJS或未知项目模式无条件报违规。两者来自额外module语料，不改写原358例六项差异。

原JavaScript/Go/Ruby归档测试写固定仓库历史文件，本批改为显式CODEGUARD_NATIVE_DIFFERENTIAL_REPORT_DIR，未指定时写各语言/进程独立临时目录；没有覆盖旧输入或报告。执行前后29份历史原生JSON摘要一致，七份本轮输入/报告用新名称归档，字段schema与来源保持。三份原生差分报告和三份corpus输入通过精确版本schema。普通相关7通过/5忽略与显式原生12不重复相加为精度；WASM CLI全目标严格Clippy、分层、OpenSpec strict、diff通过。

独立holdout、完整原生矩阵、语法版本覆盖、真实宿主、关闭、发行及CI证据源仍缺，资格0/32，父任务不勾选。原始日志、工具入口及报告摘要见evidence/native-current-nine-language-2026-10-06.json。
