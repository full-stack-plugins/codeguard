//! 已固定的 CodeGraph grammar 候选资产；这里只核对来源和字节，不声明语法能力已验收。

use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const CODEGRAPH_COMMIT: &str = "40f112453583a2304c4b605a3a9d6545919662bd";
const CODEGRAPH_LICENSE_SHA256: &str =
    "e6d98f98c666bebe065ac2492a0a19232cc318d4d67bac3ca42ffb77bacc8809";
const JAVA_COMMIT: &str = "94703d5a6bed02b98e438d7cad1136c01a60ba2c";
const TYPESCRIPT_COMMIT: &str = "f975a621f4e7f532fe322e13c4f79495e0a7b2e7";

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
    /// 上游 grammar 仓库。
    pub grammar_repository: String,
    /// 上游 grammar 完整提交。
    pub grammar_commit: String,
    /// CodeGraph 记录的上游版本标签。
    pub grammar_version: String,
    /// 相对于 grammars/ 的固定制品路径。
    pub path: String,
    /// WASM 原始字节摘要。
    pub sha256: String,
    /// WASM 原始字节长度。
    pub bytes: usize,
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

/// 读取仓内固定清单，并检查 CodeGraph 许可证及三份资产的原始字节。
pub fn bundled_grammar_candidates() -> Result<GrammarAssetManifest, String> {
    let manifest = parse_grammar_asset_manifest(include_bytes!("../../../grammars/manifest.json"))?;
    let codegraph_license = include_bytes!("../../../grammars/LICENSE.codegraph");
    if digest(codegraph_license) != CODEGRAPH_LICENSE_SHA256 {
        return Err("CodeGraph 许可证字节与固定来源不符".into());
    }
    for asset in &manifest.assets {
        let (wasm, license) = match asset.language.as_str() {
            "java" => (
                include_bytes!("../../../grammars/java/parser.wasm").as_slice(),
                include_bytes!("../../../grammars/java/LICENSE").as_slice(),
            ),
            "typescript" => (
                include_bytes!("../../../grammars/typescript/parser.wasm").as_slice(),
                include_bytes!("../../../grammars/typescript/LICENSE").as_slice(),
            ),
            "tsx" => (
                include_bytes!("../../../grammars/tsx/parser.wasm").as_slice(),
                include_bytes!("../../../grammars/typescript/LICENSE").as_slice(),
            ),
            _ => return Err("候选语言未知".into()),
        };
        verify_grammar_asset(asset, wasm, license)?;
    }
    Ok(manifest)
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
        || manifest.build_tool != "tree-sitter-cli 0.25.10 build --wasm"
        || manifest.assets.len() != 3
    {
        return Err("grammar 清单版本、来源或资产数量不符".into());
    }
    let languages: BTreeSet<&str> = manifest
        .assets
        .iter()
        .map(|asset| asset.language.as_str())
        .collect();
    if languages != BTreeSet::from(["java", "typescript", "tsx"]) {
        return Err("grammar 语言资产缺失或重复".into());
    }
    for asset in &manifest.assets {
        let (repo, commit, version, wasm_path, wasm_sha, wasm_bytes, license_path, license_sha) =
            match asset.language.as_str() {
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
                _ => return Err("grammar 语言未知".into()),
            };
        if asset.dialect != asset.language
            || asset.grammar_repository != repo
            || asset.grammar_commit != commit
            || asset.grammar_version != version
            || asset.path != wasm_path
            || asset.sha256 != wasm_sha
            || asset.bytes != wasm_bytes
            || asset.license != license_path
            || asset.license_sha256 != license_sha
            || asset.abi_version != 14
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
