# pip-audit 原生协议解析与命令计划验收

OpenSpec 对应 `introduce-rust-codeguard-cli` 的 7.2 和 `native-tool-adapters`。依据 [pip-audit 当前 JSON formatter 源码](https://github.com/pypa/pip-audit/blob/main/pip_audit/_format/json.py)核对顶层 `dependencies`/`fixes`、逐包 `name`/`version`/`vulns`、跳过包 `skip_reason`、advisory ID/别名/修复版本。官方 [CLI 选项与退出码说明](https://github.com/pypa/pip-audit/blob/main/README.md?plain=1)用于固定命令计划和 0/1 结果判定。README 中旧式数组示例不是当前 formatter 的顶层结构；解析器只接受当前对象结构。

Rust `PipAuditCommand` 只返回 `--locked` 标准锁项目模式的字面参数，并显式要求 `--strict`、JSON、别名开启、描述关闭、无进度输出与项目外私有缓存。计划不含 `--fix`、`--ignore-vuln`、`--skip-editable` 或 shell；相对路径和项目内缓存被拒。此 API 不执行进程，执行器仍须验证工具字节、项目输入、环境变量、网络来源与缓存边界。

`parse_pip_audit_json` 递归拒绝重复键，要求预期/实测 2.x 工具版本一致、无 `fixes`、无跳过依赖、每个组件名唯一、原生退出 0/1 与漏洞列表一致。结构完整时保留实际包版本、原生 advisory ID、别名和仅语法有效的 CVE 别名；不转述描述文案。空列表也返回 `advisory_coverage=not_evaluated`，不能据此声称数据库新鲜或项目无漏洞。

`crates/codeguard-adapters/tests/pip_audit_contract.rs` 先因缺解析 API、后因缺命令计划 API 两次 RED，随后六项普通协议测试通过，覆盖有效漏洞、空结果、跳过、异常退出、版本不符、重复键、歧义组件、意外修复和不安全路径。测试 JSON 按官方源码构造，**没有实际运行 pip-audit**；还未验证真实工具版本、PEP 751 环境选择、锁文件完整包图、数据库身份/时效、原生错误流、网络失败或智能体任务同步。此阶段不勾选 7.2。
