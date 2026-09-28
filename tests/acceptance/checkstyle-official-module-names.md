# Checkstyle 官方模块名称验收

状态：固定 10.21.4 的既有静态注释模块局部适配，不代表所有模块、项目配置或门禁完成。

## 行为

Checker、TreeWalker 支持官方完整类名。五类已支持 Javadoc 检查支持短名、带 Check 后缀的短名及官方完整检查类名；精确固定映射归一身份，原 XML 字节保留并交给原工具，不猜测任意包名或后缀。

属性上下文、默认 native source、修复依据与重复 source 检查使用同一归一身份。短名与完整名描述同一规则时，共享稳定 finding/task；未区分的重复 source 拒绝，不按顺序归属。相似未知包名仍返回配置上下文未完成，不自动白名单或降级执行。

## 验证

TDD：官方完整名称配置原先返回 None，绑定测试失败；实现后五类原名/Check 后缀/完整类名映射相等，跨包冒名及短名/完整名重复声明被拒。

19 项适配绑定/配置/原生结果/XML 回归、9 项普通 CLI/工作台回归通过（共 28 项）。

真实 Java 21/Checkstyle 10.21.4 四次回放通过（21.95 秒）：JavadocVariable 短名、Checker/TreeWalker/检查器完整类名及带 Check 后缀短名均产生同一个原生字段问题；首次 new_findings=1，其后为 0，无配置 blocker，始终只有一张任务。最后 task verify 为 still_present 并保存事件。每轮 scope/tokens 原属性均执行，原生 source 固定完整检查类。

其余四种 Javadoc 模块本轮验证的是固定名称绑定契约，不能把字段回放冒充四者全部原生行为验收。

```bash
cargo test -p codeguard-adapters --test checkstyle_binding_contract --test checkstyle_config_contract --test checkstyle_result_contract --test checkstyle_xml_contract
cargo test -p codeguard-cli --test java_checkstyle_cli --test java_checkstyle_workbench
CODEGUARD_JAVA_BIN=/absolute/java CODEGUARD_CHECKSTYLE_JAR=/absolute/checkstyle-10.21.4-all.jar cargo test -p codeguard-cli --test java_checkstyle_workbench official_module_aliases -- --ignored
```

## 剩余范围

自定义模块与 packageNames、外部资源、属性插值、完整 Maven/Gradle 生效配置、全部检查类/版本和可信工具/策略及正式关闭仍待完成。未知上下文不能通过短名猜测放行；6.3 保持未勾选。

最后检查：CLI 与 adapter 全目标 Clippy（-D warnings）、格式、OpenSpec 严格校验与插件差异空白检查通过。
