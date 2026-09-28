# 完整内存归档树与 bundle 身份绑定

对应 OpenSpec S05/5.3 的完整包内容基础；正式安装未完成。

新增 UnpackedArchive 与 unpack_package_archive_tree，保留显式空目录并补全所有父目录。旧 unpack_package_archive 仍返回普通文件集合，委托同一完整解析；文件/目录及隐式父目录大小写冲突都在返回前拒绝。完整树公开字段可被调用方构造或改写，因此 hash/verify 每次重新核对规范路径、重复/类型/大小写冲突、父目录及资源边界。

hash_unpacked_bundle_tree/verify_unpacked_bundle_tree 使用既有本机 codeguard-bundle-tree-v1：域标签、D/F、UTF-8 路径字节长度/内容、文件大小与 SHA-256，按本机 Path 排序子项、同层目录入栈并以后进先出继续。最多 100000 树条目、512 MiB 总文件、128 MiB 单文件；内容按 128 KiB 块核验剩余预算/取消。输入顺序不改变摘要，非零小写预期摘要才可比较。成功不表示来源批准、工具可运行或依赖闭包完整。

三项契约最初因 API 缺失失败。实施后完整树 3 项通过；包含多层目录和空目录的内存树实际写入临时目录，再与现有 CLI hash_bundle_tree 精确交叉比较。打乱内存输入顺序仍相同；修改库、删除空目录、伪造路径、缺父目录、大小写冲突、非法/零预期摘要及取消/到期均拒绝或不匹配。两种真实归档 fixture 另验证空目录/隐式父目录保留及旧文件接口不变；不同文件名下的隐式目录大小写冲突也被拒绝。

本轮归档 19、包流 9、缓存发布 10、现有制品身份 8、crate 边界 5、完整树 3，共 54 项通过。1 项原生缓存用例 ignored 不计运行；无原生工具启动。该交叉比较仅在本机执行，Windows/其它平台尚未实测。

最终 all-target Clippy（-D warnings）、cargo fmt --check、OpenSpec strict 与 git diff --check 均通过。

包内前缀到锁 bundle 根的正式映射、同一批准来源绑定、目录/整包原子发布、实际下载、CLI apply/恢复及宿主端到端仍未接通；未来落盘必须复核真实文件系统路径归一化、权限和内容，不能凭内存摘要跳过。全 workspace 未运行，不勾选 5.3/5.6。
