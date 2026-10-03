# 原生版本探测诊断验收

对应 OpenSpec S03/S05 的版本执行基础。observe_native_version 已接入现有 Ruff 检查启动阶段；doctor CLI 尚未完成，不能称其已发布。

统一 Rust runtime 服务接收固定版本参数、工具字节摘要、精确 stdout、共同截止时间和本轮私有日志。拒绝源码扫描参数/stdin；入口与执行后工具内容均核对。只在零退出、精确输出、无未预期 stderr、内容不变、日志成功且仍在预算内时返回完整本地观察。结果不授予策略批准、工作区准备就绪或质量通过。

超时、启动前预算耗尽、取消、无法启动、信号退出、输出超限、管道/清理/平台故障和非零原生退出分别保留诊断码及执行类别。版本输出不符、stderr 异常、工具内容变化与日志失败分开。原始 stdout/stderr 不进入观察结果，仅留私有日志。日志失败仍保留已执行进程的 termination；不能把零退出当成完整。

Ruff 复用该服务，版本不完整时不会继续源码扫描；此前的 version_execution_incomplete 合并诊断已移除。共享截止时间不重置，版本与扫描使用同一个预算。

```bash
CODEGUARD_TEST_RUFF=/opt/anaconda3/bin/ruff cargo test -p codeguard-cli --test native_version_observation --test ruff_probe_contract --test python_lint_scan_contract --test crate_boundaries --offline
cargo test -p codeguard-runtime --test process_contract --test private_log_contract --offline
CODEGUARD_RUFF_BIN=/opt/anaconda3/bin/ruff cargo test -p codeguard-cli --test ruff_probe_contract --test python_lint_scan_contract --offline -- --ignored
```

目标用例先因类型/函数缺失失败。7 项探测用例覆盖精确输出、版本/stderr/退出差异、时间/取消、未匹配身份/非版本参数不启动、日志失败、spawn/signal/输出超限/执行期间字节变化，以及显式选择的真实 Ruff 0.16.8 版本调用。macOS 测试临时根路径含链接时日志守卫拒绝；改用规范化路径，未放宽安全约束。自改测试最初追加了非法命令而非注释，造成非零退出；改成合法注释后才能验证“零退出但字节变化”的目标行为。

7 项探测中的真实 Ruff 用例仅证明版本诊断。默认目标及 runtime process/private log/边界共 43 项通过；另外明确设置 CODEGUARD_RUFF_BIN，补跑两套各 5 项 ignored 真实扫描回归，10 项全部通过。这些扫描验证现有局部 Ruff 配置/范围/报告，不证明完整门禁。all-target Clippy、格式、OpenSpec strict 与 diff 检查通过。

完整 doctor、可信工具锁、原生兼容与运行环境、其它适配器复用、准备报告持久化/同步仍缺；5.2/5.6 不勾选，完整计划保持进行中。

2026-10-03 的 [CI 37112015676](https://github.com/full-stack-plugins/codeguard/actions/runs/37112015676) 在 30ms 超时样例中得到 `SpawnFailure` 而非 `TimedOut`。运行时正确保留了独立的启动失败类别；失败点是测试在并行 Linux 负载下未能真正启动睡眠进程。测试现给启动过程 500ms，令受测进程睡眠 2 秒，并仅对瞬时 `SpawnFailure` 使用新的私有夹具重试最多两次；最终仍必须观察到真正的 `TimedOut`，不能把启动失败当成超时通过。本机定向测试 1/1、该文件全部 7/7 通过，远端重新验收待最新 PR CI 完成。
