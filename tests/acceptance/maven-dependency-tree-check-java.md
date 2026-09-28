# Maven 依赖图进入 `check java` 的局部原生观察

对应 OpenSpec `introduce-rust-codeguard-cli` 6.8 的局部执行进展。`check java`/`check all` 对每个静态声明 Maven Dependency Plugin 的构建根建立 `java.dependencies` 任务，包括无 `.java` 源文件的根。只在原 POM 属于无父 POM、profile、扩展、外部仓库与额外插件的静态简单项目时，把 POM 复制到私有目录，用显式 Maven/JDK/离线仓库身份执行插件 3.8.1 的 `tree -DoutputType=json`。执行前后核对原 POM、私有副本、Maven/JDK 与仓库树摘要；解析 JSON 根项目、组件和传递边。human/JSON 反馈保留原生状态及组件，交付仍未评估。Maven goal 失败、报告损坏、根项目错配、来源/工具变化等只形成 `incomplete`，不报告空图为清洁。

真实反例发现 Maven 在离线制品已存在但 `_remote.repositories` 来源与隔离设置镜像不一致时，可以打印“POM missing”警告却返回 `BUILD SUCCESS`，依赖图只含直接 JUnit、丢失 Hamcrest。探针现在要求本轮恰有一次预期 goal 与成功标记，并拒绝所有原生 warning/error 和来源不可用提示。测试专用的约 420 MiB 离线仓库补齐插件闭包及 JUnit 的镜像来源元数据后，显式 Maven 3.9.16/JDK 21 用例从 `check java` 得到 JUnit 4.13.2 → Hamcrest Core 1.3 的传递边。该元数据修正仅用于隔离验收仓库，不属于 CodeGuard 对用户缓存的自动修复。

定向普通测试覆盖受控进程调度、私有工作目录、JSON/human 输出、静态 POM 门禁与成功但缺 POM 的日志反例。真实用例需显式提供 `CODEGUARD_MAVEN_BIN`、`CODEGUARD_JAVA_HOME` 和 `CODEGUARD_DEP_MAVEN_REPO`，且仓库树须在 512 MiB 预算内并具有完整离线闭包。私有工作目录尚不等于系统调用级文件/网络沙箱：外部进程仍可能访问原项目或网络，3.11 的可信执行边界未验收。此用例只证明本机局部原生执行，不证明所有 Maven 模块、生效模型、私服、动态版本、许可证、SBOM 或 CVE 数据库；`coverage_proven=false`，6.8 仍未完成。

后续回归发现，仅按 `.java` 源文件调度会漏掉尚无源码、但已声明 Maven Dependency Plugin 和依赖的 POM；父根有源码时也会遗漏无源码的子构建根。现在依赖图任务按每个已配置的构建根调度，P3C/Javadoc 的源码条件保持不变。两个 CLI 回归样本先重现遗漏，修复后分别确认零源码根仍得到 `prerequisites_missing` 的具体探针结果，以及父、子两根均列入待探针列表。它们不证明真实多模块 Maven 生效模型已覆盖；无工具前置条件仍为 `native_incomplete`，不能变成空图通过。

`check_feedback` 0.12 的依赖图探针 0.2 为能从固定离线仓库精确定位的非根制品增加 `artifact_sha256`。路径由已解析的 Maven group/artifact/version/type/classifier 构造，拒绝可越界坐标；只读取普通有界文件，找不到或不支持的制品为 `null`，不推断摘要。读取后再核对仓库树摘要，避免把仓库变化后的值当作原扫描身份。一个模拟 Maven/JDK/仓库的 CLI 测试先因缺字段失败，实施后确认报告中摘要等于 JAR 实际字节；真实 Maven 仓库的制品摘要覆盖尚未逐项验收。该值是本地观察，不单独证明仓库可信，也不证明 OWASP 报告使用了同一制品。
