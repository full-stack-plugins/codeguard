# 原生差分当前语料绑定修复

日期：2026-10-05；对应 OpenSpec 14.17、14.19 局部验收。

远端 CI 37260434907 及本机回归复现两个失败：原生差分仍使用绑定旧清单的 0.2 语料，生产校验正确拒绝 `grammar_evaluation_corpus_identity_invalid`。本次为测试入口增加显式重绑定：先按保存的 2026-10-04 清单验证历史，再只替换清单摘要、核对全部 cases 未变，并通过当前生产校验。历史语料原字节保留，生产侧不接受旧身份。额外测试明确核对旧原生语料在工具执行前被拒绝。

重绑定后暴露 Ruff 控制测试的过期 argv：旧替身要求 `--isolated` 首位和相对 stdin 文件名，当前隔离入口实际使用绝对 `/codeguard_input.py`、独立最后 `--isolated`。更新替身的精确全参数断言，仍验证 E9、py312、禁缓存、忽略 noqa、JSON 和 stdin；不放宽原生报告解析。

显式实际 Ruff 对照的输出改为独立 `python-native-grammar-differential-current-manifest-2026-10-05.json`，不得覆盖旧报告。原生工具由显式环境变量提供，不下载、不安装；控制替身测试不算实际原生精度验收。

## 本机实际原生证据

显式 `/opt/anaconda3/bin/ruff` 0.16.8 对 18 个 Python 样例执行独立隔离语法检查，全部获得可判定原生结果、没有标签分歧，程序身份稳定。与 grammar 对照为 6 TP、10 TN、2 FN；漏报仍是既有 `python-empty_body` 与 `python-bad_indent`，不能称作语法精度通过。报告保持 `incomplete`、`not_evaluated`、grammar 资格数 0。

新报告 SHA-256 `dae43030040a933719fc7b6435abca007fae834b20b824687d97c5ec73bc9f91`，本地加载所有引用 schema 后通过 Draft202012 校验。旧报告与旧语料原字节独立核对不变。这个结果修复回放入口，不消除已知 grammar 漏报。

包含显式实际 Ruff 对照的完整原生差分目标以 `--include-ignored --test-threads=1` 执行：6 项通过、零失败、零忽略，206.32 秒。控制回归与真实原生检查分别保留各自证据范围；新提交的远端 CI 尚待终态。
