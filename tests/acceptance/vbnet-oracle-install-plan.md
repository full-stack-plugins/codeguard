# VB.NET grammar 修复所需工具安装方案（等待用户批准）

对应现有 OpenSpec 14.4 / 14.17 / 14.19；此文件不表示已安装或验收完成。

当前缺少 tree-sitter 生成器和 .NET。仅在批准后下载到 `/Users/wandl/Library/Caches/codeguard/acceptance-tools/`，不修改系统 PATH、Shell 配置、全局 npm/Cargo、不使用 sudo，也不发布 Codeguard。已有工作区与 grammar 资产保持原样，后续候选重建须单独保留来源及回归证据。

## 已核验的固定来源

- Tree-sitter CLI **0.25.10**，与当前 VB.NET 候选的构建来源一致；macOS arm64 压缩二进制约 5.9 MB。GitHub release 元数据给出 SHA-256：`a6295d469669f6e901b6029ae6c129bd094df582d278c237fada413505cb9c42`。[固定发行](https://github.com/tree-sitter/tree-sitter/releases/tag/v0.25.10)、[官方 CLI 安装说明](https://tree-sitter.github.io/tree-sitter/creating-parsers/1-getting-started.html)。
- .NET SDK **10.0.401**，当前官方 active LTS 10.0 的具体 SDK 版本，仅用于隔离原生对照。macOS arm64 archive SHA-512：`69f64eb00dc045398755c440b152225d544301a345a146a16e86a56a0c52b7c94b2c331520e976dbb821f18d31930aafbd25bb85961e3517e0665414ce0cbcff`。[官方发行元数据](https://builds.dotnet.microsoft.com/dotnet/release-metadata/10.0/releases.json)、[官方 macOS 安装说明](https://learn.microsoft.com/en-us/dotnet/core/install/macos)。下载及解包空间尚未实测，安装前检查可用空间；不将该 SDK 范围宣称为 Codeguard 已支持全部 VB.NET 版本。

## 具体命令（批准后才执行）

以下命令使用独立固定路径；目标安装目录若已存在，先核验并重用，不能覆盖未知内容。

```bash
mkdir -p /Users/wandl/Library/Caches/codeguard/acceptance-tools/downloads
curl --fail --location 'https://github.com/tree-sitter/tree-sitter/releases/download/v0.25.10/tree-sitter-macos-arm64.gz' --output /Users/wandl/Library/Caches/codeguard/acceptance-tools/downloads/tree-sitter-0.25.10.gz
printf '%s  %s\n' 'a6295d469669f6e901b6029ae6c129bd094df582d278c237fada413505cb9c42' '/Users/wandl/Library/Caches/codeguard/acceptance-tools/downloads/tree-sitter-0.25.10.gz' | shasum -a 256 --check
mkdir /Users/wandl/Library/Caches/codeguard/acceptance-tools/tree-sitter-0.25.10
gzip --decompress --stdout /Users/wandl/Library/Caches/codeguard/acceptance-tools/downloads/tree-sitter-0.25.10.gz > /Users/wandl/Library/Caches/codeguard/acceptance-tools/tree-sitter-0.25.10/tree-sitter
chmod u+x /Users/wandl/Library/Caches/codeguard/acceptance-tools/tree-sitter-0.25.10/tree-sitter
/Users/wandl/Library/Caches/codeguard/acceptance-tools/tree-sitter-0.25.10/tree-sitter --version

curl --fail --location 'https://builds.dotnet.microsoft.com/dotnet/Sdk/10.0.401/dotnet-sdk-10.0.401-osx-arm64.tar.gz' --output /Users/wandl/Library/Caches/codeguard/acceptance-tools/downloads/dotnet-sdk-10.0.401-osx-arm64.tar.gz
printf '%s  %s\n' '69f64eb00dc045398755c440b152225d544301a345a146a16e86a56a0c52b7c94b2c331520e976dbb821f18d31930aafbd25bb85961e3517e0665414ce0cbcff' '/Users/wandl/Library/Caches/codeguard/acceptance-tools/downloads/dotnet-sdk-10.0.401-osx-arm64.tar.gz' | shasum -a 512 --check
mkdir /Users/wandl/Library/Caches/codeguard/acceptance-tools/dotnet-10.0.401
tar -xzf /Users/wandl/Library/Caches/codeguard/acceptance-tools/downloads/dotnet-sdk-10.0.401-osx-arm64.tar.gz -C /Users/wandl/Library/Caches/codeguard/acceptance-tools/dotnet-10.0.401
DOTNET_CLI_TELEMETRY_OPTOUT=1 DOTNET_SKIP_FIRST_TIME_EXPERIENCE=1 /Users/wandl/Library/Caches/codeguard/acceptance-tools/dotnet-10.0.401/dotnet --info
```

校验不通过则停止，不解包或执行。官方工具安装成功不授予 grammar 资格：仍须隔离编译器对合法未缩进、合法缩进及真正非法样例作对照，再修复固定 grammar 来源并重建；不能忽略全部 `MISSING ':'`、补用户缩进或改历史标签。Tree-sitter 生成后可复用现有 Zig 编译链，不能让 build 命令隐式下载另一套 SDK。
