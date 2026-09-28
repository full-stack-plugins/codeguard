# PMD 6 原生命令参数的局部验收（2026-09-24）

PMD 官方 [6.15.0 安装与启动文档](https://raw.githubusercontent.com/pmd/pmd/pmd_releases/6.15.0/docs/pages/pmd/userdocs/installation.md)说明 `run.sh pmd` 原生入口；[同版本 CLI 参考](https://raw.githubusercontent.com/pmd/pmd/pmd_releases/6.15.0/docs/pages/pmd/userdocs/cli_reference.md)列出 `-d`、`-R`、`-f`、`-r`、`-showsuppressed` 与 `-no-cache`。`Pmd6Command` 以此构造只读的字面 argv，但 PMD 6.15.0 与 P3C 的实际组合仍需原生执行验证。

`codeguard-adapters` 的命令规划只接受一个绝对源码路径、一个单一规则集引用和一个绝对报告路径，拒绝相对/含父目录路径、逗号串联规则集及 URL 规则源。报告目标在唯一 `-r` 参数中显式声明，同时固定 XML、显示 suppression、禁用增量缓存。局部试运行服务已将同一报告路径交给 Rust runtime 的新鲜报告槽位，见 [PMD 6 原生试运行的局部证据](pmd6-runtime-probe-baseline.md)；工具发行包、P3C 规则制品、JDK 及完整项目扫描范围仍须另行核验。

`cargo test -p codeguard-adapters --test pmd6_command_contract --offline` 仅验证参数构造。它不是 PMD 6.15.0/P3C 真实兼容或规则加载证明，不能据此完成 OpenSpec 5.4/6.2 或宣布 Java lint 可用。Windows `pmd.bat` 入口尚未适配。
