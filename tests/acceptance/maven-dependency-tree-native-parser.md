# Maven 原生依赖图 JSON 解析基线

对应 OpenSpec `introduce-rust-codeguard-cli` 6.8 的局部适配器进展。Rust `codeguard-adapters::parse_maven_dependency_tree_json` 读取 Maven Dependency Plugin 3.7+ 的 `tree` JSON，保留根项目、传递依赖与每条父子边；不把相同组件出现在不同路径时合并。输入大于 4 MiB、结构不完整、未知字段、动态/越界坐标、未知 scope、布尔形状错误、过深或过多节点均拒绝，不将坏报告当作空图。依赖图只证明原生工具报告的组件关系，不包含 CVE、许可证、SBOM、版本新旧或数据库时效结论。

目标测试先因解析器不存在而编译失败，实现后普通 4 项通过。本机显式真实测试通过：Maven 3.9.16 离线调用 `org.apache.maven.plugins:maven-dependency-plugin:3.8.1:tree -DoutputType=json -DoutputFile=tree.json`，解析 JUnit 4.13.2 到 Hamcrest Core 1.3 的传递边。运行命令：

```bash
cargo test -p codeguard-adapters --test maven_dependency_tree_contract --offline
CODEGUARD_MAVEN_BIN=/opt/homebrew/Cellar/maven/3.9.16/bin/mvn cargo test -p codeguard-adapters --test maven_dependency_tree_contract --offline -- --ignored
```

本条记录对应解析器阶段；后续已把静态简单 POM 的原生 goal 接入私有运行时和 `check java` 反馈，见 `maven-dependency-tree-check-java.md`。外部仓库、动态版本、私服解析、完整模块覆盖与 Maven 生效配置仍未验收；6.8 继续未完成。
