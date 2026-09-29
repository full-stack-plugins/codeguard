# npm 0.1.1 单平台候选来源绑定

对应 OpenSpec `introduce-rust-codeguard-cli` 的 S11.1、S13.4 与 `binary-distribution`。

2026-09-29：`version_cli` 先在显式 `CODEGUARD_BUILD_SHA` 下因 `build_identity=null` 失败；实现后 2 项通过。源码版本与锁同步到 0.1.1。公开 npm 打包器新增干净 checkout 和候选构建提交等于 Git HEAD 的要求：当前工作树未提交时，公开打包在写出候选包之前拒绝。普通本地包仍允许无构建提交声明，不能借此获得公开发行身份。

本次源码完整工作区 `cargo test --workspace --all-features -- --test-threads=1 -q` 退出 0；无效构建提交 `CODEGUARD_BUILD_SHA=not-a-commit` 的版本目标测试 2 项通过且报告 null。全目标 Clippy `-D warnings`、改动 Rust 文件的 rustfmt、Node 语法、OpenSpec strict 与 diff 检查通过。先前脏工作树中制作的本地 0.1.1 预演包只用于本机安装和 `PostToolUseFailure` 命令验证，不是正式候选包。

候选构建提交来自编译环境，本身可被伪造；干净 checkout 和自报匹配也不能证明可复现构建、发行签名、平台矩阵或宿主实际绑定。

公开候选从干净的 `f8311d63f29b76f7740e37d6689884f9ac490ca4` 构建：`CODEGUARD_BUILD_SHA=<HEAD> cargo build --release --locked --offline -p codeguard-cli`，随后执行 `node scripts/pack-npm-local.mjs --public target/release/codeguard`。本机 tarball 为 `codeguard-public-0.1.1-darwin-arm64.tgz`，SHA-256 `65bcca8d5647d3618d8795970fe689ffabd76ecc835cf5dd3e2b63084fa8be83`；原生程序 SHA-256 `0399ed58ca602c37fc7b1a7bf98803166a5a9271481d842da615acf3e7767671`。本地全新安装的 `--version --format=json` 回报 `cli_version=0.1.1`、`target=macos_arm64` 和该源码提交。GitHub [CI 36506889265](https://github.com/full-stack-plugins/codeguard/actions/runs/36506889265) 对同一提交的构建、Linux WASM 资源与全套测试均成功；它不是本机发布制品的可复现构建证明。

`npm publish ./release/npm/codeguard-public-0.1.1-darwin-arm64.tgz --access public --tag latest --ignore-scripts` 退出 0。注册表随后显示 `latest: 0.1.1`，并给出 SHA-512 integrity `sha512-NdhFwz7zGwfZEtxUzGh95eZ3aQraoZDvkqC7SoK2v5rD1bSBt9WodU6TDauU0nCObIxj3SqhWQsnrjkXlPQjGA==`。初期 metadata 已可见而 tarball 暂时 404；2026-09-29 01:25 UTC 后 tarball 返回 200。全新 npm 缓存的 `npx --yes @partme.ai/codeguard@0.1.1 --version --format=json` 退出 0，并回报同一版本、平台和构建提交。独立 `npm pack` 从注册表取得的包与本机候选 tarball 均为上述 SHA-256；包内二进制也与上述原生程序 SHA-256 相同。发布可安装性和该候选的字节一致性在 macOS arm64 上成立。

此证据尚不满足可信签名发行、可复现构建、多平台矩阵、插件 runtime lock、实际宿主绑定和完整质量门禁；S11.1、S13.4 继续不勾选。
