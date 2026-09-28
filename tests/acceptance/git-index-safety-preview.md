# Git index 路径安全预览切片

Rust Core 的 `check_repository_paths` 复现旧插件的敏感文件名、构建产物目录和仓根转储规则，同时保留 `.github/workflows`、嵌套 `scripts/vendor` 和测试 fixture 中 `.db` 的合法情形。它只判断路径政策，文件名命中不等于检测到真实密钥。非规范路径返回错误，不被默认为安全。

Rust CLI 的 `observe_index_safety` 通过受控原生 Git 进程读取仓库根的真实 `ls-files --stage -z --cached`；支持 SHA-1/SHA-256 OID 格式、替代 `GIT_INDEX_FILE`、未合并阶段拒绝和前后两次 index 列表一致性检查。它通过 `git cat-file --batch-check/--batch` 读取暂存 blob，独立重算 Git OID，并记录脱敏的内容 SHA-256、长度和类型；每个对象上限 8 MiB，总长度上限 128 MiB。symlink、gitlink、LFS 指针及超出读取预算的对象保持 unresolved；读取错误时仍保留独立取得的路径违规。暂存后工作树变化不会替代 index 中的字节。

`codeguard gate pre-commit` 目前只提供此路径与对象预览：JSON/human 报告实际暂存违规和对象核验状态，始终返回 3/incomplete/not_evaluated。公开 schema 为 0.2.0。Git 工具身份、完整质量义务及受保护政策尚未验证；前后列表相同也不是原子 index 快照，不能把没有路径违规或普通 blob 已核验解释为交付通过。

目标测试先因 Rust 模块缺失失败；随后验证暂存 `.env`、未暂存 `.pem`、替代 index、默认 index 不变、真实 SHA-1/SHA-256 仓库、暂存后工作树改动、symlink/LFS、超预算对象仍保留路径违规、坏记录/冲突、非 Git 目录、公开 CLI JSON/退出码及坏参数，并校验公开预览 schema 不含 allow 值。`git_index_safety_contract` 共 13 项通过，`repository_path_safety_contract` 共 3 项通过。真实 Git Hook、pre-push 多 ref、CI 与三宿主接线不在此切片内，OpenSpec 3.5/3.6/4.6/11.5/12.12 保持未完成。
