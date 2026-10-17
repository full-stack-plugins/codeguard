# 格式化器档案分片契约（format batch schema）

> 每个子智能体只写自己负责的一个分片文件：`rulepacks/format_batches/<batch>.json`。
> 文件所有权互不重叠，可安全并行。最后由合并步骤统一生成 `rulepacks/format_profiles.json`。
> **不要修改 `rulepacks/format_profiles.json`、`crates/codeguard-cli/src/format_command.rs`
> 或任何其他分片文件。**

## 分片文件结构

```json
{
  "schema_version": "0.1.0",
  "batch": "<batch名>",
  "languages": [ { ...语言档案... } ]
}
```

## 语言档案字段（字段名和类型必须完全一致）

| 字段 | 类型 | 说明 |
|---|---|---|
| `language` | string | 注册表语言 ID，必须与 `rulepacks/legacy_languages.json` 的 `id` 逐字一致 |
| `status` | string | 固定 `integrated`（本分片任务都是已接入）；无法给出真实工具时用 `not_integrated` |
| `formatter` | string | 格式化器可执行文件名，如 `google-java-format`、`ktlint`、`stylua` |
| `tool_key` | string | 与 formatter 相同（供 `--tool NAME=ABS_PATH` 绑定） |
| `check_argv` | string[] | 只读检查参数模板，必须用 `{file}` 占位 |
| `apply_argv` | string[] | 就地格式化参数模板，必须用 `{file}` 占位 |
| `config_markers` | string[] | 项目配置文件名（不含路径），如 `pyproject.toml`；无则 `[]` |
| `extensions` | string[] | 源文件扩展名，**带前导点**，如 `.java` |
| `dialects` | object | 方言/版本键值；无则 `{}` |
| `notes` | string | 中文说明，必须写清该工具的适用范围与限制 |

## 退出码语义（最重要，违反会导致集成失败）

运行时 `format check` 用如下规则判定，**check_argv 必须落在 A 或 B 之一**：

- **A 类（stdout 列出不合规文件）**：`gofmt -l`、`prettier --list-different`、`black --check`
  - 退出 0 且 stdout 为空 → 合规
  - 退出 0 且 stdout 非空 → 不合规（文件路径逐行列出）
- **B 类（退出码 1 表示不合规）**：`clang-format --dry-run --Werror`、`google-java-format --dry-run --set-exit-if-changed`
  - 退出 0 → 合规
  - 退出 1 → 不合规
  - 退出 ≥2 → 工具故障（视为未完成，不得当合规）

`apply_argv` 统一语义：**退出 0 → 成功格式化**。

## 已验证可用的参考实现

以下 7 种语言已在主档案中接入，**照抄它们的写法**：

```json
{
  "language": "rust",
  "status": "integrated",
  "formatter": "rustfmt",
  "tool_key": "rustfmt",
  "check_argv": ["--check", "--edition", "{edition}", "{file}"],
  "apply_argv": ["--edition", "{edition}", "{file}"],
  "config_markers": ["rustfmt.toml", ".rustfmt.toml"],
  "extensions": [".rs"],
  "dialects": {"edition": "2021"},
  "notes": "rustfmt 是 Rust 官方格式化器；--check 只读检查不改源码"
}
```

B 类参考（clang-format）：

```json
"check_argv": ["--dry-run", "--Werror", "--style=file", "{file}"],
"apply_argv": ["-i", "--style=file", "{file}"]
```

## 工具选择原则

1. **优先该语言的官方/事实标准格式化器**（如 gofmt、rustfmt、dart format、mix format），
   其次生态主流工具（ktlint、stylua、shfmt、swift-format），再次通用工具（clang-format、prettier）。
2. **不要为同一语言选两种工具**。一条语言档案只对应一个格式化器。
3. 若某语言生态**没有公认格式化器**（例如 CFML、CUDA、Metal），
   用 `status: "not_integrated"`，`formatter`/`tool_key`/`check_argv`/`apply_argv` 填 `null`，
   `extensions` 填 `[]`，并在 `notes` 写明「该生态无公认格式化器，format 检查如实不适用」。
   **这不算失败，如实披露优于编造。**
4. `planned` 三种语言（arkts/cobol/metal）一律 `not_integrated`，理由写明工具链未稳定或超限。

## 硬性要求

- `extensions` 必须带前导点（`.java` 不是 `java`）
- `check_argv` 与 `apply_argv` 都必须含 `{file}` 占位
- `notes` 必须是中文，且要说明**该工具不能做什么**（例如「只判格式，不检查注释完整性」）
- JSON 用 UTF-8、`ensure_ascii=false`、2 空格缩进
- 分片内 `languages` 的 `language` 不得重复
