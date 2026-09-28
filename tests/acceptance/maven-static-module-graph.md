# Maven 静态模块图

范围：OpenSpec 9.18/9.23 的 Maven 直接声明切片；完整有效模型、其它生态、源码引用与条件解析仍缺，不勾选整项。

`MavenModuleModel` 在同次有界清单读取上解析直接坐标、modules 和 dependencies，不执行 Maven、wrapper 或脚本。图 0.2 保留目录 contains，分列 aggregation 和 declared build_dependency。每条 Maven 关系有来源清单/原始字节 SHA-256 和声明条件；依赖另含 scope。只有源/目标完整且唯一的直接 group/artifact/version 才形成本地依赖边。属性变量、父 POM、profile、管理版本、特殊属性、重复坐标、无本地目标及模块路径越界显式 unresolved，不猜测有效依赖，也不把 profile 模块列为直接聚合。依赖完整性始终 false，不用于裁剪检查。

协议 0.1 历史 schema 保留为 module-graph-v0.1.schema.json。当前初始化生产 0.2，不从旧图推断新关系。新增产物改变受管摘要时沿已有受控画像刷新；用户工作记录保留。

新增 init 用例首先因旧图只有 0.1/contains 而失败；实现后测试区分两类 Maven 声明、精确摘要、重复/条件/变量/逃逸反例，以及依赖版本变化后删除旧关系且保留人工任务备注、重复刷新幂等。适配器测试覆盖命名空间、坏 XML、重复字段、256KiB 上限、缺/变量坐标、optional/classifier/type/scope/systemPath，以及 XML 注释切分字段文本不得误取首段。

真实 CLI 初始化产物经 JSON Schema 验证；六个反例（把 declared 改成 observed、坏摘要、错误条件/scope、缺来源、把依赖覆盖改为 true）均拒绝，历史 schema 正例仍有效。验证辅助使用现有 jsonschema 环境；产品观察、图生成和测试入口是 Rust，没有添加 Python 产品实现。

受影响 init/detect/plan/config/边界和 CLI 库测试、adapter 测试均通过。静态检查曾发现导出位于测试模块后，第一次调整遗漏导出，已恢复正确位置并通过；分段文本新增反例先失败，修复后 adapter 4 项、受影响 CLI 66 项再次通过，最终 workspace all-target Clippy（-D warnings）和格式检查通过。OpenSpec strict/diff 校验通过。未执行全 workspace/native 全语言/真实 Maven 生效模型或端到端质量交付。
