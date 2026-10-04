# Zig 原生工具发现与当前输入

对应已有 OpenSpec `introduce-rust-codeguard-cli` 9.9/9.10、14.10/14.11/14.19 的原生优先与复检契约。完整聚合Zig原生接线、完整lint/build、全语言和发行父任务仍未完成。

`lint zig` 与 Zig `task verify` 共用显式优先/PATH选择。空、相对目录和不可执行文件不作为自动入口；第一个可执行工具的版本失败不改用后面的编译器。保留原生失败的未验收WASM补充不是原生通过。单文件报告独立升级0.2，记录选择来源和当前源码；0.1原件不改。工具解析目标在原生执行前冻结；源文件或入口目标变化撤回旧原生位置，源变化不保留旧字节WASM。输入检查没有替代完整进程沙箱或所有瞬时文件变化证明。

## RED与回归

原入口PATH已有Zig仍输出工具未提供，任务复检新增反例也返回not_run：两个真实CLI受控测试先失败后通过。原生探针返回诊断期间修改源码的反例也先失败，旧入口保留了过时坐标；现在报告source_current=false及输入阻塞，撤回坐标。受控目标另覆盖工具别名变化、显式无效工具不执行PATH候选、首选版本失败不运行后续编译器、相对/空/不可执行PATH目录。

默认两个目标7 passed / 0 failed / 1条件忽略；首次相关WASM五目标29 passed / 0 failed / 3条件忽略。最终工具冻结修改后的相关回归另补终态，不把早期结果当作最终源码全套结果。缺工具测试显式清空PATH；保留另一个自动发现测试，不依赖开发机偶然安装状态。

## 实际已安装Zig0.16.0

- 单文件PATH入口实测1 passed / 0 failed / 0 ignored，合法`const Empty = struct {};`为原生completed，非法`pub fn main( void {`为diagnostics_observed，均不运行WASM。两份报告见[实际lint](evidence/zig-path-lint-2026-10-05.json)。该测试4.27秒为整组观察，不是性能达标证据。
- 原生任务显式/PATH两种模式各执行非法与修复样本，实测1 passed / 0 failed / 0 ignored，同一任务、同一规范工具路径、事件真实追加；修复观察为candidate_absent_unverified_policy，事实仍open。四轮任务链路测试构建耗时95.67秒，不作为优化后的发行性能。见[实际复检](evidence/zig-path-task-2026-10-05.json)。
- 4项开发期协议/状态负例检查通过：真实lint0.2、旧消费者拒绝、新协议撤回过时坐标、拒绝有诊断的completed和无工具来源的原生完成、真实复检兼容旧协议及open状态。

```bash
cargo test --locked --offline -p codeguard-cli --features wasm-precheck \
  --test zig_native_discovery --test zig_lint_cli --test syntax_task_verify
CODEGUARD_ZIG_BIN=/absolute/path/to/zig cargo test --locked --offline \
  -p codeguard-cli --features wasm-precheck --test zig_native_discovery \
  real_zig_on_path_runs_native_ast_check -- --ignored --exact
```

没有安装工具，没有变更规则批准或门禁；公开npm0.1.4及插件锁没有升级。AST单文件检查不是完整Zig lint或构建，32grammar资格不因工具发现接线改变。

## 最终源码验证

工具目标冻结后的相关WASM八目标52 passed / 0 failed / 6条件忽略；Hook十五项另15 passed / 0 failed / 0 ignored。Hook的缺工具反例先因开发机已安装Zig而暴露旧断言，再以明确空PATH验证缺工具；自动发现另由独立反例覆盖。真实工具测试单独显式执行，不把条件忽略计为验收通过。上述目标有覆盖重叠，不累加为全套结果。

默认及WASM workspace/all-targets严格Clippy、278文档本地文件链接、4协议负例、fmt、分层、OpenSpec strict通过；243历史schema字节全部保留，新0.2独立加入。此前全套默认1288结果对应旧提交，本轮没有借用它证明当前全套回归。
