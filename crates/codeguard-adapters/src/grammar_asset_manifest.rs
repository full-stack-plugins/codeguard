//! 已固定的 CodeGraph grammar 候选资产；这里只核对来源和字节，不声明语法能力已验收。

use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const CODEGRAPH_COMMIT: &str = "1072f82ce24db3d133258d30165cef6b74d108b2";
const CODEGRAPH_LICENSE_SHA256: &str =
    "e6d98f98c666bebe065ac2492a0a19232cc318d4d67bac3ca42ffb77bacc8809";
const JAVA_COMMIT: &str = "94703d5a6bed02b98e438d7cad1136c01a60ba2c";
const TYPESCRIPT_COMMIT: &str = "f975a621f4e7f532fe322e13c4f79495e0a7b2e7";
const PYTHON_COMMIT: &str = "bffb65a8cfe4e46290331dfef0dbf0ef3679de11";
const ZIG_COMMIT: &str = "b670c8df85a1568f498aa5c8cae42f51a90473c0";
const C_COMMIT: &str = "b780e47fc780ddc8da13afa35a3f4ed5c157823d";
const GO_COMMIT: &str = "3c3775faa968158a8b4ac190a7fda867fd5fb748";
const JAVASCRIPT_COMMIT: &str = "44c892e0be055ac465d5eeddae6d3e194424e7de";
const RUST_COMMIT: &str = "77a3747266f4d621d0757825e6b11edcbf991ca5";
const CPP_COMMIT: &str = "f41e1a044c8a84ea9fa8577fdd2eab92ec96de02";
const CSHARP_COMMIT: &str = "cac6d5fb595f5811a076336682d5d595ac1c9e85";
const LUA_COMMIT: &str = "816840c592ab973500ae9750763c707b447e7fef";
const LUAU_COMMIT: &str = "a8914d6c1fc5131f8e1c13f769fa704c9f5eb02f";
const ARKTS_COMMIT: &str = "56f7fc288715befe92c734d603f9e6bc3c65d2a8";
const NIX_COMMIT: &str = "3d0173d903e630b6e14d17f1cf79488791379ded";
const TERRAFORM_COMMIT: &str = "fad991865fee927dd1de5e172fb3f08ac674d914";

/// 代码来源、许可和每份候选 grammar 的固定身份。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GrammarAssetManifest {
    /// 本清单格式。
    pub schema_version: String,
    /// CodeGraph 上游仓库。
    pub source_repository: String,
    /// CodeGraph 上游不可变提交。
    pub source_commit: String,
    /// 随本仓保留的 CodeGraph 许可证。
    pub codegraph_license: String,
    /// 该许可证原始字节的摘要。
    pub codegraph_license_sha256: String,
    /// 上游记录的构建工具版本与调用；只作来源声明。
    pub build_tool: String,
    /// CodeGraph 锁定的依赖包完整性；适用于 Objective-C 与 Solidity。
    pub dependency_package_integrity: Option<String>,
    /// 依赖包随附许可证路径。
    pub dependency_package_license: Option<String>,
    /// 依赖包许可证摘要。
    pub dependency_package_license_sha256: Option<String>,
    /// 精确且无重复的候选语言资产。
    pub assets: Vec<GrammarAsset>,
}

/// 单份 grammar 的来源、编译制品及尚未验证的语言范围。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GrammarAsset {
    /// 语言规范 ID。
    pub language: String,
    /// 待验收的方言 ID。
    pub dialect: String,
    /// 与外部语言 ID 不同时，固定 WASM 的 tree_sitter_* 导出名。
    pub loader_symbol: Option<String>,
    /// 上游 grammar 仓库。
    pub grammar_repository: String,
    /// 上游 grammar 完整提交。
    pub grammar_commit: String,
    /// CodeGraph 记录的上游版本标签。
    pub grammar_version: String,
    /// 对预编译 npm 资产固定包完整性；其它来源为空。
    pub package_integrity: Option<String>,
    /// 相对于 grammars/ 的固定制品路径。
    pub path: String,
    /// WASM 原始字节摘要。
    pub sha256: String,
    /// WASM 原始字节长度。
    pub bytes: usize,
    /// 可重现适配前的依赖包原始路径。
    pub source_wasm: Option<String>,
    /// 可重现适配前的原始摘要。
    pub source_sha256: Option<String>,
    /// 相对于 grammars/ 的许可证路径。
    pub license: String,
    /// 许可证原始字节摘要。
    pub license_sha256: String,
    /// CodeGraph 上已观察的 Tree-sitter ABI；Rust 兼容仍需另验。
    pub abi_version: u32,
    /// CodeGraph 加载所用的运行时版本。
    pub codegraph_runtime: String,
    /// CodeGuard Rust 加载状态，尚不能由静态清单升格为通过。
    pub codeguard_runtime_validation: String,
    /// 已在 CodeGuard 验收的语言版本；当前必须为空。
    pub language_versions: Vec<String>,
    /// 本候选的已知缺口。
    pub known_limitations: Vec<String>,
    /// 发行状态；候选字节不能等同已发行可用。
    pub release_status: String,
}

/// 读取仓内固定清单，并检查来源、许可证及候选资产字节。
pub fn bundled_grammar_candidates() -> Result<GrammarAssetManifest, String> {
    let manifest = parse_grammar_asset_manifest(include_bytes!("../../../grammars/manifest.json"))?;
    verify_codegraph_license()?;
    verify_dependency_license(&manifest)?;
    for asset in &manifest.assets {
        verify_bundled_asset(asset)?;
    }
    Ok(manifest)
}

/// 只核对指定内置 grammar 的字节与许可，供按需加载的语法工作进程使用。
/// 参数为固定语言 ID；返回清单身份和静态 WASM 字节，未知语言返回错误。
pub fn bundled_grammar_candidate(language: &str) -> Result<(GrammarAsset, &'static [u8]), String> {
    let manifest = parse_grammar_asset_manifest(include_bytes!("../../../grammars/manifest.json"))?;
    let asset = manifest
        .assets
        .iter()
        .find(|asset| asset.language == language)
        .ok_or("不支持的 grammar 语种")?;
    verify_codegraph_license()?;
    if matches!(language, "objc" | "solidity") {
        verify_dependency_license(&manifest)?;
    }
    let wasm = verify_bundled_asset(asset)?;
    Ok((asset.clone(), wasm))
}

fn verify_codegraph_license() -> Result<(), String> {
    if digest(include_bytes!("../../../grammars/LICENSE.codegraph")) != CODEGRAPH_LICENSE_SHA256 {
        return Err("CodeGraph 许可证字节与固定来源不符".into());
    }
    Ok(())
}

fn verify_dependency_license(manifest: &GrammarAssetManifest) -> Result<(), String> {
    if digest(include_bytes!(
        "../../../grammars/LICENSE.tree-sitter-wasms"
    )) != manifest
        .dependency_package_license_sha256
        .as_deref()
        .unwrap_or("")
    {
        return Err("依赖包许可证字节与固定来源不符".into());
    }
    Ok(())
}

fn verify_bundled_asset(asset: &GrammarAsset) -> Result<&'static [u8], String> {
    let (wasm, license) = match asset.language.as_str() {
        "arkts" => (
            include_bytes!("../../../grammars/arkts/parser.wasm").as_slice(),
            include_bytes!("../../../grammars/arkts/LICENSE").as_slice(),
        ),
        "c" => (
            include_bytes!("../../../grammars/c/parser.wasm").as_slice(),
            include_bytes!("../../../grammars/c/LICENSE").as_slice(),
        ),
        "cpp" => (
            include_bytes!("../../../grammars/cpp/parser.wasm").as_slice(),
            include_bytes!("../../../grammars/cpp/LICENSE").as_slice(),
        ),
        "csharp" => (
            include_bytes!("../../../grammars/csharp/parser.wasm").as_slice(),
            include_bytes!("../../../grammars/csharp/LICENSE").as_slice(),
        ),
        "go" => (
            include_bytes!("../../../grammars/go/parser.wasm").as_slice(),
            include_bytes!("../../../grammars/go/LICENSE").as_slice(),
        ),
        "javascript" => (
            include_bytes!("../../../grammars/javascript/parser.wasm").as_slice(),
            include_bytes!("../../../grammars/javascript/LICENSE").as_slice(),
        ),
        "lua" => (
            include_bytes!("../../../grammars/lua/parser.wasm").as_slice(),
            include_bytes!("../../../grammars/lua/LICENSE").as_slice(),
        ),
        "luau" => (
            include_bytes!("../../../grammars/luau/parser.wasm").as_slice(),
            include_bytes!("../../../grammars/luau/LICENSE").as_slice(),
        ),
        "nix" => (
            include_bytes!("../../../grammars/nix/parser.wasm").as_slice(),
            include_bytes!("../../../grammars/nix/LICENSE").as_slice(),
        ),
        "java" => (
            include_bytes!("../../../grammars/java/parser.wasm").as_slice(),
            include_bytes!("../../../grammars/java/LICENSE").as_slice(),
        ),
        "typescript" => (
            include_bytes!("../../../grammars/typescript/parser.wasm").as_slice(),
            include_bytes!("../../../grammars/typescript/LICENSE").as_slice(),
        ),
        "python" => (
            include_bytes!("../../../grammars/python/parser.wasm").as_slice(),
            include_bytes!("../../../grammars/python/LICENSE").as_slice(),
        ),
        "tsx" => (
            include_bytes!("../../../grammars/tsx/parser.wasm").as_slice(),
            include_bytes!("../../../grammars/typescript/LICENSE").as_slice(),
        ),
        "zig" => (
            include_bytes!("../../../grammars/zig/parser.wasm").as_slice(),
            include_bytes!("../../../grammars/zig/LICENSE").as_slice(),
        ),
        "objc" => (
            include_bytes!("../../../grammars/objc/parser.wasm").as_slice(),
            include_bytes!("../../../grammars/objc/LICENSE").as_slice(),
        ),
        "solidity" => (
            include_bytes!("../../../grammars/solidity/parser.wasm").as_slice(),
            include_bytes!("../../../grammars/solidity/LICENSE").as_slice(),
        ),
        "rust" => (
            include_bytes!("../../../grammars/rust/parser.wasm").as_slice(),
            include_bytes!("../../../grammars/rust/LICENSE").as_slice(),
        ),
        "terraform" => (
            include_bytes!("../../../grammars/terraform/parser.wasm").as_slice(),
            include_bytes!("../../../grammars/terraform/LICENSE").as_slice(),
        ),
        _ => return Err("候选语言未知".into()),
    };
    verify_grammar_asset(asset, wasm, license)?;
    if asset.language == "zig"
        && crate::adapt_zig_wasm(include_bytes!("../../../grammars/zig/source.wasm"))? != wasm
    {
        return Err("Zig grammar 来源适配关系不符".into());
    }
    if matches!(asset.language.as_str(), "objc" | "solidity") {
        let source = if asset.language == "objc" {
            include_bytes!("../../../grammars/objc/source.wasm").as_slice()
        } else {
            include_bytes!("../../../grammars/solidity/source.wasm").as_slice()
        };
        if asset.source_sha256.as_deref() != Some(digest(source).as_str())
            || crate::adapt_legacy_dylink(&asset.language, source)? != wasm
        {
            return Err("依赖 grammar 来源适配关系不符".into());
        }
    }
    Ok(wasm)
}

/// 解析来源固定的候选清单；拒绝重复字段、漂移来源和伪造的已验收状态。
pub fn parse_grammar_asset_manifest(raw: &[u8]) -> Result<GrammarAssetManifest, String> {
    if raw.is_empty() || raw.len() > 64 * 1024 {
        return Err("grammar 清单为空或过大".into());
    }
    let strict = crate::strict_json::parse_unique_json(raw).map_err(str::to_owned)?;
    let manifest: GrammarAssetManifest =
        serde_json::from_value(strict).map_err(|error| error.to_string())?;
    if manifest.schema_version != "1.0.0"
        || manifest.source_repository != "https://github.com/partme-ai/codegraph"
        || manifest.source_commit != CODEGRAPH_COMMIT
        || manifest.codegraph_license != "LICENSE.codegraph"
        || manifest.codegraph_license_sha256 != CODEGRAPH_LICENSE_SHA256
        || manifest.build_tool
            != "tree-sitter-cli 0.25.10 build --wasm; Zig: tree-sitter-cli 0.27.0 generate --abi 15 + Zig 0.16.0 wasm32-wasi; ArkTS/Terraform: pinned npm prebuilt WASM"
        || manifest.dependency_package_integrity.as_deref()
            != Some(
                "tree-sitter-wasms@0.1.13 sha512-wT+cR6DwaIz80/vho3AvSF0N4txuNx/5bcRKoXouOfClpxh/qqrF4URNLQXbbt8MaAxeksZcZd1j8gcGjc+QxQ==",
            )
        || manifest.dependency_package_license.as_deref() != Some("LICENSE.tree-sitter-wasms")
        || manifest.dependency_package_license_sha256.as_deref()
            != Some("6b0382b16279f26ff69014300541967a356a666eb0b91b422f6862f6b7dad17e")
        || manifest.assets.len() != 18
    {
        return Err("grammar 清单版本、来源或资产数量不符".into());
    }
    let languages: BTreeSet<&str> = manifest
        .assets
        .iter()
        .map(|asset| asset.language.as_str())
        .collect();
    if languages
        != BTreeSet::from([
            "c",
            "arkts",
            "cpp",
            "csharp",
            "go",
            "java",
            "javascript",
            "lua",
            "luau",
            "nix",
            "objc",
            "python",
            "rust",
            "solidity",
            "terraform",
            "typescript",
            "tsx",
            "zig",
        ])
    {
        return Err("grammar 语言资产缺失或重复".into());
    }
    for asset in &manifest.assets {
        let (repo, commit, version, wasm_path, wasm_sha, wasm_bytes, license_path, license_sha) =
            match asset.language.as_str() {
                "arkts" => (
                    "https://github.com/harmony-contrib/tree-sitter-arkts",
                    ARKTS_COMMIT,
                    "0.2.0",
                    "arkts/parser.wasm",
                    "db0812971109457d22b3fe9dcdb1cd8e614fba2a338e220670bd254485753623",
                    4384161,
                    "arkts/LICENSE",
                    "048e4dcb7a71fb725495ebb3e7052d0e76dcb6705b828af8b9b461df10bcd8ca",
                ),
                "c" => (
                    "https://github.com/tree-sitter/tree-sitter-c",
                    C_COMMIT,
                    "v0.24.2",
                    "c/parser.wasm",
                    "a271e584616c7c3c0ac663f01cd05dd5f1a6c2ce6d4cd23096548985a95d0ccb",
                    625716,
                    "c/LICENSE",
                    "2e0110e07abef7c2548b26ec9d6969775617ca539a0dc8dbeeb14d6452c711d1",
                ),
                "cpp" => (
                    "https://github.com/tree-sitter/tree-sitter-cpp",
                    CPP_COMMIT,
                    "v0.23.4",
                    "cpp/parser.wasm",
                    "70f5e2b9976dad56bdcd1fafcb3af8c839c7a92e7beaa437162bcf45f390e83d",
                    3434644,
                    "cpp/LICENSE",
                    "2e0110e07abef7c2548b26ec9d6969775617ca539a0dc8dbeeb14d6452c711d1",
                ),
                "csharp" => (
                    "https://github.com/tree-sitter/tree-sitter-c-sharp",
                    CSHARP_COMMIT,
                    "v0.23.5",
                    "csharp/parser.wasm",
                    "6f69e1cae44e1c32c1eccc170dc5a9778fb94ff716f71113fe1f8c4299aa2f40",
                    5350581,
                    "csharp/LICENSE",
                    "778fb7d63b8c1844da315648c02f325c4713a6f4a5d19644fd413421422776d3",
                ),
                "go" => (
                    "https://github.com/tree-sitter/tree-sitter-go",
                    GO_COMMIT,
                    "v0.23.4",
                    "go/parser.wasm",
                    "4eda5d91c99ca981e88bc7d3d33f0db166b4bab0a84d0021a9abf39b364c78ef",
                    210013,
                    "go/LICENSE",
                    "2e0110e07abef7c2548b26ec9d6969775617ca539a0dc8dbeeb14d6452c711d1",
                ),
                "java" => (
                    "https://github.com/tree-sitter/tree-sitter-java",
                    JAVA_COMMIT,
                    "v0.23.5",
                    "java/parser.wasm",
                    "181a6fbc34d7864a551d91c13882fc33007e923b3c81a4bdbe7fa47492090077",
                    414653,
                    "java/LICENSE",
                    "52ed137b039cd9c46409bc22e89938af911c95b157feae2d040b51e6084369a7",
                ),
                "python" => (
                    "https://github.com/tree-sitter/tree-sitter-python",
                    PYTHON_COMMIT,
                    "v0.23.6",
                    "python/parser.wasm",
                    "a7fdc587e77bd729b9f5b783c659be23c896e305a2c374472bed7114d9e01fac",
                    456131,
                    "python/LICENSE",
                    "d724405ce238a22c0d35769c5a36b386ad5958192efe8bbb304fb2896254575f",
                ),
                "javascript" => (
                    "https://github.com/tree-sitter/tree-sitter-javascript",
                    JAVASCRIPT_COMMIT,
                    "v0.25.0",
                    "javascript/parser.wasm",
                    "7978e62bcc851ab1d1f6dcd4678f9eda79df2b3b3490e75f81dd819d9bccccfa",
                    411832,
                    "javascript/LICENSE",
                    "2e0110e07abef7c2548b26ec9d6969775617ca539a0dc8dbeeb14d6452c711d1",
                ),
                "lua" => (
                    "https://github.com/tree-sitter-grammars/tree-sitter-lua",
                    LUA_COMMIT,
                    "v0.4.1",
                    "lua/parser.wasm",
                    "6d95607fc7d78964cfdf065ccb1ba76be5ed217c5ec0d0a3cace13c59fa1ae43",
                    49488,
                    "lua/LICENSE",
                    "9a32b02e4c917b1ce6b5e79d8ea81e25cefd7f27d89c7235f2afb262c06cf32e",
                ),
                "luau" => (
                    "https://github.com/tree-sitter-grammars/tree-sitter-luau",
                    LUAU_COMMIT,
                    "v1.2.0",
                    "luau/parser.wasm",
                    "f1647052518f2bdfae8e8c0b033ffdeca1193d69d11c78ba20f84c8374fd0fe3",
                    94204,
                    "luau/LICENSE",
                    "099c44248f8cf353123211318680e93465587c005a4b3730ce8cb5334de043d6",
                ),
                "nix" => (
                    "https://github.com/nix-community/tree-sitter-nix",
                    NIX_COMMIT,
                    "3d0173d+codegraph-cli-0.25.10",
                    "nix/parser.wasm",
                    "4acffa1c013df751193a21ce429777ca214c7d3a7e3665085b6df00dad8869f0",
                    80876,
                    "nix/LICENSE",
                    "c1b72e50266464ec5118d26200e17bf1f472d142e62ab637e1ffc23986657a78",
                ),
                "rust" => (
                    "https://github.com/tree-sitter/tree-sitter-rust",
                    RUST_COMMIT,
                    "v0.24.2",
                    "rust/parser.wasm",
                    "206031e0f67fb41ecae505868ca3bb917df7375031aebafd2f97314a849713fe",
                    1114303,
                    "rust/LICENSE",
                    "31d5b6f4243d5c7c6e1c4ebbbb9f6407bd1457a08bcc4f706521710341acba36",
                ),
                "typescript" => (
                    "https://github.com/tree-sitter/tree-sitter-typescript",
                    TYPESCRIPT_COMMIT,
                    "v0.23.2",
                    "typescript/parser.wasm",
                    "3a44d634c9840dccec36f33b99592bd086a2f940e9cd80c64347c45b7dce662f",
                    1414055,
                    "typescript/LICENSE",
                    "49bf33cf78ef5897e4e161ce1517df7de1ae5042a65b6bcfd44401e0fc606559",
                ),
                "tsx" => (
                    "https://github.com/tree-sitter/tree-sitter-typescript",
                    TYPESCRIPT_COMMIT,
                    "v0.23.2",
                    "tsx/parser.wasm",
                    "8f647a1b2cafe9ab00fb2056d79021d2a144ba17a72f45511072311c1b05d08e",
                    1445641,
                    "typescript/LICENSE",
                    "49bf33cf78ef5897e4e161ce1517df7de1ae5042a65b6bcfd44401e0fc606559",
                ),
                "zig" => (
                    "https://github.com/tree-sitter-grammars/tree-sitter-zig",
                    ZIG_COMMIT,
                    "1.1.2+codegraph-empty-containers-patch",
                    "zig/parser.wasm",
                    "e8a3aa89cc07b59188122e6c1e0a812ca58cf85c9e191dcf2663a30b6b3b99e3",
                    705328,
                    "zig/LICENSE",
                    "0eea8dc45e89deeb03c7799bbbc7b4688f365fb274562f4540ecfebdea82e727",
                ),
                "objc" => (
                    "https://github.com/amaanq/tree-sitter-objc",
                    "dea2b2d6a1253c2f0f58b543c10d62edd2f860f2",
                    "2.1.0",
                    "objc/parser.wasm",
                    "2606d4c5809fab61de44d328072d1371d53aa4aff5734436cb2a0ce8db7f2b0c",
                    7708267,
                    "objc/LICENSE",
                    "099c44248f8cf353123211318680e93465587c005a4b3730ce8cb5334de043d6",
                ),
                "solidity" => (
                    "https://github.com/JoranHonig/tree-sitter-solidity",
                    "b239a95f94cfcc6e7b3e961bc73a28d55e214f02",
                    "1.2.0",
                    "solidity/parser.wasm",
                    "ba02ba3c98c8ce976ed962d727ef48940b3a18dd2243830a8158b558de64b4f2",
                    423943,
                    "solidity/LICENSE",
                    "8844f0cc9b76b9c8a9d0251904eb7536b6bb9976e0ec577e8f27ab96d42523ef",
                ),
                "terraform" => (
                    "https://github.com/tree-sitter-grammars/tree-sitter-hcl",
                    TERRAFORM_COMMIT,
                    "1.2.0",
                    "terraform/parser.wasm",
                    "0d9ef3ae926acc0411bfecba9b34df4ea659061917b96455570a45a70824e890",
                    92484,
                    "terraform/LICENSE",
                    "c71d239df91726fc519c6eb72d318ec65820627232b2f796219e87dcf35d0ab4",
                ),
                _ => return Err("grammar 语言未知".into()),
            };
        if asset.dialect != asset.language
            || asset.package_integrity.as_deref()
                != match asset.language.as_str() {
                    "arkts" => Some(
                        "tree-sitter-arkts@0.2.0 sha512-j4KpZ21YdX5koXiuuslML24LoKLOzV6uZ1PMG//c/ynAoQfD7XY2LWnFxJiX0sOLqf2pOEYHdUmKFnXjm4QL4g==",
                    ),
                    "terraform" => Some(
                        "@tree-sitter-grammars/tree-sitter-hcl@1.2.0 sha512-2bVnOojkkdMLevp0G4v3ksbNoOQFc/Pt9GAdWX4i3aykVyI+CkktE1hsF/XAeUQFjwgGrVZnEyeCll5oD7Ibfg==",
                    ),
                    _ => None,
                }
            || asset.loader_symbol.as_deref()
                != if asset.language == "csharp" {
                    Some("c_sharp")
                } else {
                    None
                }
            || (match asset.language.as_str() {
                "objc" => {
                    asset.source_wasm.as_deref() != Some("objc/source.wasm")
                        || asset.source_sha256.as_deref()
                            != Some(
                                "7c1b5bfdca7e64b6c63b6040bb7ba0afc347df116f9030ca32f8535d7377f6ff",
                            )
                }
                "solidity" => {
                    asset.source_wasm.as_deref() != Some("solidity/source.wasm")
                        || asset.source_sha256.as_deref()
                            != Some(
                                "160745e470f234cae903a9ba445d19e758d0b02e1197401fc765976c6254d2b6",
                            )
                }
                _ => asset.source_wasm.is_some() || asset.source_sha256.is_some(),
            })
            || asset.grammar_repository != repo
            || asset.grammar_commit != commit
            || asset.grammar_version != version
            || asset.path != wasm_path
            || asset.sha256 != wasm_sha
            || asset.bytes != wasm_bytes
            || asset.license != license_path
            || asset.license_sha256 != license_sha
            || asset.abi_version
                != if matches!(
                    asset.language.as_str(),
                    "zig" | "c" | "csharp" | "javascript" | "lua" | "rust" | "nix"
                ) {
                    15
                } else {
                    14
                }
            || asset.codegraph_runtime != "web-tree-sitter 0.25.3"
            || asset.codeguard_runtime_validation != "rust_loader_smoke_passed"
            || !asset.language_versions.is_empty()
            || asset.known_limitations.is_empty()
            || asset.known_limitations.iter().any(String::is_empty)
            || asset.release_status != "candidate_unvalidated"
        {
            return Err(format!("{} grammar 来源或验收状态不符", asset.language));
        }
    }
    Ok(manifest)
}

/// 校验调用者提供的制品与许可字节；只返回候选身份，不证明 Rust 可加载或语法精度。
pub fn verify_grammar_asset(
    asset: &GrammarAsset,
    wasm: &[u8],
    license: &[u8],
) -> Result<(), String> {
    let manifest = parse_grammar_asset_manifest(include_bytes!("../../../grammars/manifest.json"))?;
    if !manifest.assets.iter().any(|known| known == asset) {
        return Err("grammar 身份未列入固定清单".into());
    }
    if wasm.len() != asset.bytes
        || !wasm.starts_with(b"\0asm\x01\0\0\0")
        || digest(wasm) != asset.sha256
        || license.is_empty()
        || digest(license) != asset.license_sha256
    {
        return Err("grammar 或许可证字节与固定摘要不符".into());
    }
    Ok(())
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
