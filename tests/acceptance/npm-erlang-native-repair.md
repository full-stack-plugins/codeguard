# npm 安装后的 Erlang 原生发现与修复验收

日期：2026-10-04。对应唯一 OpenSpec change `introduce-rust-codeguard-cli` 的 binary-distribution 安装后原生首次任务场景，关联 8.134 / 11.17 / 13.4 / 14.18。此记录证明本机 macOS arm64 私有候选包的命令链路，不证明公开发行升级、完整 Erlang 项目、grammar 精度或已安装宿主自动对话。

## 旧发行对照

旧 `target/release/codeguard` 的程序 SHA-256 `34c2490ff4ccaa04bce9e91efd6ccabff6d679e97b08966b1edba9b6aec37266` 与已发布 [0.1.4 验收记录](npm-0.1.4-candidate.md) 一致，`build_identity=1cd458f6e01a44a74388243e964e3f45290ac18e`。新测试通过真实 packer 和离线 npm 安装该旧程序后，在 `lint erlang` 上退出2、明确拒绝不支持的入口。该 RED 证明旧发行缺少能力，不是当前源码新增缺陷。

## 当前安装后的可观察行为

[Node 验收目标](../npm_pack_erlang_repair.test.mjs) 用现有 WASM binary 生成私有 `@full-stack-plugins/codeguard` 包，通过隔离缓存中的 `npm exec --offline --ignore-scripts` 执行，不发布、不全局安装。检查项目与 npm 缓存/受控工具分开；第一次试跑把缓存置于项目内，额外扫描范围使单源计数断言失败，这是测试基础设施污染，不记为产品精度缺陷。

受控协议与显式真实 OTP 28 两组都实际验证：

- `init` 后直接 `lint erlang` 创建原生首次任务，没有先执行 WASM，也不伪造 grammar 身份。
- `check all` 复用同一任务，当前原生覆盖跳过重复候选解析；`next` 引用已保存报告，报告摘要与当前字节相符。
- human 输出携带同一任务；`task verify` 保存原工具诊断，源码变化后 next 清空陈旧位置。
- 修复后的零原生诊断保存为 `candidate_absent_unverified_policy`，首次事实仍 open，不自行批准关闭。
- 问题再次出现复用原任务，`repair_ready` 经安装后的 launcher stdin 回传原生位置、字符列单位和真实保存引用。
- 另一工作台的 reports 路径被普通文件占用时，原生诊断保留、task_id 为 null、同步原因具体可见。

最终 Node runner 3 passed / 0 failed / 0 skipped，43.24秒：其中1项是父级组织测试，2项是上述独立场景，不宣称3个独立原生能力。未指定真实 OTP 的 CI 路径另运行，2 passed / 0 failed / 1 skipped；该跳过不算原生验收。

实际导出的两组共26份结构化报告通过相应 schema，14个伪造变体被拒绝，包括虚构任务、原生报告伪造 grammar、错来源引用、假 resolved、零诊断与非空位置矛盾、Hook allow 和保存失败却带任务。受控协议与真实原生证据分开，不混合统计精度。

## 可复现命令

```bash
cargo build --locked --offline -p codeguard-cli --features wasm-precheck
CODEGUARD_WASM_BIN="$PWD/target/debug/codeguard" \
CODEGUARD_ERL_BIN=/absolute/path/to/erl \
CODEGUARD_NPM_ERLANG_ARTIFACT=/absolute/path/observations.json \
node --test tests/npm_pack_erlang_repair.test.mjs
```

`CODEGUARD_ERL_BIN` 必须指向明确选择的真实 OTP 28；不提供时该场景明确 skipped。导出路径可省略。当前源码 binary SHA-256 `f281f88cd5db6cd18657cf31c84f3079a733456ec1a42a9c1f11846d9397b7ca`，私有 tarball SHA-256 `c96d42382d922e779911e7a95006bb056182f2146b86b0c643483447a13bdb53`。debug构建未声明发行提交，不能把相同0.1.4版本号当作注册表已升级。该包只用于本批安装验收。

CI 在旧全 grammar 包和 Zig 修复包测试后顺序运行新目标，共享输出不并发覆盖。Linux 的远端结果须按包含新目标的提交独立核验。本机新增 Node 测试没有改变 Rust 产品源码；2ff4b91 对应的1172项默认、51项相关特性和2项真实 OTP 回归不重记为本批新测试。

## 剩余范围

公开 npm 0.1.4 字节及插件锁未改变。完整 Erlang 项目/宏/include/条件编译/注释、可信关闭与复发重开、独立语料精度、真实宿主、MSRV、多平台及完整门禁仍未完成；已知10个 grammar 漏检保留。8.134 / 11.17 / 13.4 / 14.18 父任务继续开放。

日志及报告身份：

- `/tmp/codeguard-npm-erlang-old-release-red.log`：SHA-256 `35e7de3da1ba5b7cc5b40d09d7a3e4f3494dfac1489d26a4f35bc5f2538bffcd`。
- `/tmp/codeguard-npm-erlang-current.log`：SHA-256 `6b579080ebfbb5777c9ba8e6a6e75a1473671d05aa00695479f8aa2a5d6e2614`。
- `/tmp/codeguard-npm-erlang-current-observations.json`：SHA-256 `b3817f717a94417b5d23168fa58efed16f0f0f224e62f754c61494e571bf3e86`。
- `/tmp/codeguard-npm-erlang-ci-path.log`：SHA-256 `a3acb959d92c33cf7fa3563ec357d5d3c72ff1570bf597b1755208587d7f57ba`。
- `/tmp/codeguard-npm-erlang-schema.log`：SHA-256 `23aae0bf5d7295d505b52312b403d49dfcccbe9af13a1c168620f586cfb96d2b`。
