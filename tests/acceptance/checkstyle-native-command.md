# Checkstyle 原生参数与显式验收入口

对应 OpenSpec 6.3；当前参数规划为 Rust 纯逻辑，不执行、安装或解释配置。

CheckstyleCommand 生成固定 Java argv：-jar 本地自包含 JAR、-c 原配置快照、-f xml、-o 本轮报告、单个源码快照。全部路径须绝对且无父目录/控制字符，输入及输出不得互相覆盖；空格保持为同一字面参数，不调用 shell。文件存在性、链接/别名、制品身份、规则生效、freshness、预算及隔离须由 runtime 另证，参数有效不代表可以启动。

命令契约测试先因缺 API 编译失败，新增实现后通过。checkstyle_native_replay 提供显式 Java 与 10.21.4 all.jar 的真实测试入口；违规与文档完整两轮使用不同新报告，要求类型、自定义方法身份、缺 @param/@return 诊断与修正后零诊断。现复用统一 runtime 的私有日志/报告槽位、20 秒共同截止、清空继承环境和有界输出；临时目录退出清理。此为真实原生局部验收，尚非生产 Checkstyle 服务或正式 CLI 接线。

核对 [10.21.4 XMLLogger 源码](https://github.com/checkstyle/checkstyle/blob/checkstyle-10.21.4/src/main/java/com/puppycrawl/tools/checkstyle/XMLLogger.java) 发现自定义模块 ID 会单独写入 source；新版文档的类名#ID 不能反套旧版。解析保持原始值，实际规则绑定必须结合配置与工具版本，不从单独 ID 猜类名。

初次本机 .m2、Homebrew 与临时缓存未发现 JAR，真实测试未执行。后从官方固定 10.21.4 release 下载测试制品到 /tmp/codeguard-checkstyle-10.21.4-all.jar，大小 19631994 字节，SHA-256 为 3c1d94d6ecc83e02dff587c9ba5b6b4ec4fec38c7a958eb587efec9b28e2f318。发布 API 没有 digest 字段；该摘要是本机内容观察，不宣称独立发行批准或供应链验签。

真实测试先因缺配置 DOCTYPE 失败：Checkstyle 配置解析异常且没有报告，未产生可归属的源码 finding。修正为 10.21.4 ConfigurationLoader 映射到制品内部的标准 PUBLIC DTD 后，Java 21.0.12.1 通过统一 runtime 执行原工具，违规及补全文档两轮通过。随后加入相同诊断改为 warning 的第三轮，实际退出零但三条诊断仍保留；三轮均进入新的退出/范围判定，见 checkstyle-exit-scope-coherence.md。无独立网络隔离证明；此配置没有其它外部资源，但不能扩展为任意配置都可安全离线执行。完整 Checkstyle 生产执行/配置/项目/任务/白名单/门禁及跨平台仍缺，6.3 不勾选。
