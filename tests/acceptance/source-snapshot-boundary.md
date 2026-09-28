# 原生检查输入的有界源码快照

Rust runtime 的 `SourceSnapshot` 从调用方明确给出的相对路径集合读取普通文件，限制路径深度、文件数、单文件及总字节数；拒绝绝对路径、`..`、重复路径和符号链接。Unix 上从固定根目录描述符逐级使用 `openat` + `O_NOFOLLOW` 读取，避免检查路径类型和打开之间的目录替换把输入引到根目录之外；并发目录替换反例只能读取根内字节或返回错误。副本只能写入尚不存在的目录。原生检查之后以同样的描述符相对方式复核项目源文件和私有副本的原始字节；任一侧变化不能成为完整检查结果。

JDK Javadoc 单文件入口已接入该契约：第一次读取与快照读取不同会在执行前返回 `source_changed_before_scan`，执行后变化返回 `source_changed_during_scan`。真实 JDK 21 的有诊断与已注释样本仍通过局部验收。快照只包含调用方列明的文件，不证明源集完整；私有副本写入、整个原生进程及其插件副作用尚无内核级沙箱。项目级 Maven Javadoc 执行仍须限定 POM、生成源码、类路径和插件副作用，并证明原生产物属于本轮独立快照。

```bash
cargo test -p codeguard-runtime --test source_snapshot_contract --offline
cargo test -p codeguard-cli --test java_javadoc_cli --offline
CODEGUARD_JAVA_HOME=/absolute/path/to/jdk-21 cargo test -p codeguard-cli --test java_javadoc_cli --offline -- --ignored
```
