# PMD 6 原生试运行的局部证据（2026-09-24）

`Pmd6ProbeRequest` 把固定 `Pmd6Command` 参数交给 Rust runtime，先拒绝旧报告，再运行原生启动器、保存私有日志，并读取同轮新生成的 XML。试运行比较执行前后的源码、启动器 SHA-256 及整个 PMD 目录树摘要，拒绝启动器逃逸发行包或运行中修改被委托的 JAR；检查 XML 声明的单一文件路径、PMD 版本和原生退出码：零违规须退出 0，有违规须退出 4。任何处理错误、报告缺失、路径不符、旧报告或退出码矛盾均为 `Incomplete`，不归类成源码违规或通过。

`cargo test -p codeguard-cli --test pmd6_probe_contract --offline` 用模拟的 `run.sh` 验证八个执行契约反例和正例。模拟器只证明本地调用与报告一致性；`LocalReportCoherent` **不**代表 P3C 已加载、PMD 发行包来源可信、JDK 兼容、规则集来源可信或 Java lint 可对外交付。当前执行服务没有并入正式 `codeguard check`，已批准发行包的锁定来源、实际 P3C 正反例、覆盖证明及并发篡改隔离仍待实现，OpenSpec 5.4/6.2 保持未完成。
