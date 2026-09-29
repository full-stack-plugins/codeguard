//! CodeGraph grammar 来源覆盖清单的只读投影；不加载 WASM，也不签发检查结果。

use codeguard_adapters::bundled_grammar_candidates;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::process::ExitCode;

const SOURCE_COMMIT: &str = "40f112453583a2304c4b605a3a9d6545919662bd";
const LANGUAGES: [&str; 32] = [
    "arkts",
    "c",
    "cfml",
    "cfquery",
    "cfscript",
    "cobol",
    "cpp",
    "csharp",
    "dart",
    "erlang",
    "go",
    "java",
    "javascript",
    "kotlin",
    "lua",
    "luau",
    "nix",
    "objc",
    "pascal",
    "php",
    "python",
    "r",
    "ruby",
    "rust",
    "scala",
    "solidity",
    "swift",
    "terraform",
    "tsx",
    "typescript",
    "vbnet",
    "zig",
];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CoverageInventory {
    schema_version: String,
    source_repository: String,
    source_commit: String,
    scope: String,
    note: String,
    assets: Vec<CoverageAsset>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CoverageAsset {
    language: String,
    provider: String,
    source_path: Option<String>,
    sha256: Option<String>,
    bytes: Option<usize>,
}

/// 展示 CodeGraph 固定来源与 CodeGuard 已验证候选之间的覆盖差距。
/// 参数仅接受 `--format human|json`；返回 3 表示仍有未接入或未验收的 grammar。
pub fn run(args: &[String]) -> ExitCode {
    let format = match args {
        [] => "human",
        [option, value] if option == "--format" && matches!(value.as_str(), "human" | "json") => {
            value.as_str()
        }
        [option] if option == "--format=human" => "human",
        [option] if option == "--format=json" => "json",
        _ => {
            eprintln!("grammar status 仅支持 --format human|json");
            return ExitCode::from(2);
        }
    };
    let report = match coverage_report() {
        Ok(report) => report,
        Err(reason) => {
            eprintln!("grammar 来源清单未完成：{reason}");
            return ExitCode::from(4);
        }
    };
    if format == "json" {
        println!("{report}");
    } else {
        println!(
            "CodeGraph grammar 32 种独立资产：随仓固定 30；CodeGuard 候选 {}；已验收发行 0。",
            report["candidate_count"]
        );
        println!("全部语法初检覆盖未完成；原生 lint 义务不变。使用 --format=json 查看逐语言缺口。");
    }
    ExitCode::from(3)
}

fn coverage_report() -> Result<Value, String> {
    let raw = include_bytes!("../../../grammars/codegraph-coverage.json");
    let strict = codeguard_adapters::parse_unique_json(raw).map_err(str::to_owned)?;
    let inventory: CoverageInventory =
        serde_json::from_value(strict).map_err(|error| error.to_string())?;
    validate_inventory(&inventory)?;
    let candidates = bundled_grammar_candidates()?;
    let candidate_languages = candidates
        .assets
        .iter()
        .map(|asset| asset.language.as_str())
        .collect::<BTreeSet<_>>();
    let inventory_languages = inventory
        .assets
        .iter()
        .map(|asset| asset.language.as_str())
        .collect::<BTreeSet<_>>();
    if !candidate_languages.is_subset(&inventory_languages) {
        return Err("CodeGuard 候选不在 CodeGraph 来源清单中".into());
    }
    let assets = inventory
        .assets
        .iter()
        .map(|asset| {
            let candidate = candidates
                .assets
                .iter()
                .find(|candidate| candidate.language == asset.language);
            json!({
                "language":asset.language,
                "provider":asset.provider,
                "source_path":asset.source_path,
                "source_sha256":asset.sha256,
                "source_bytes":asset.bytes,
                "integration_status":candidate.map_or("not_integrated", |item| item.release_status.as_str()),
                "runtime_observation":candidate.map(|item| item.codeguard_runtime_validation.as_str()),
                "released":false,
                "gap":if asset.provider == "tree_sitter_wasms_dependency" {"dependency_bytes_not_pinned"}
                    else if asset.bytes.is_some_and(|bytes| bytes > 8 * 1024 * 1024) {"current_loader_size_limit"}
                    else if candidate.is_some() {"language_qualification_and_release_pending"}
                    else {"asset_provenance_license_and_loader_validation_pending"},
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "schema_version":"1.0.0",
        "report_type":"grammar_coverage_inventory",
        "source_repository":inventory.source_repository,
        "source_commit":inventory.source_commit,
        "codegraph_grammar_count":inventory.assets.len(),
        "codegraph_vendored_count":inventory.assets.iter().filter(|asset| asset.provider == "codegraph_vendored").count(),
        "candidate_count":candidates.assets.len(),
        "released_count":0,
        "authority":"source_inventory_only",
        "parser_capability":"unverified",
        "gate_effect":"none",
        "execution":"not_run",
        "delivery_decision":"not_evaluated",
        "next_action":"固定各缺口资产的上游来源与许可证，逐语言验证 Rust 加载、版本语料及原生对照；不得凭此库存跳过原生 lint。",
        "assets":assets,
    }))
}

fn validate_inventory(inventory: &CoverageInventory) -> Result<(), String> {
    if inventory.schema_version != "1.0.0"
        || inventory.source_repository != "https://github.com/partme-ai/codegraph"
        || inventory.source_commit != SOURCE_COMMIT
        || inventory.scope != "CodeGraph grammar file map excluding the jsx alias of javascript"
        || inventory.note.is_empty()
        || inventory.assets.len() != LANGUAGES.len()
    {
        return Err("CodeGraph 来源清单版本、来源或范围不符".into());
    }
    let mut seen = BTreeSet::new();
    let mut vendored = 0;
    for asset in &inventory.assets {
        if !seen.insert(asset.language.as_str()) {
            return Err("CodeGraph grammar 重复".into());
        }
        match asset.provider.as_str() {
            "codegraph_vendored" => {
                vendored += 1;
                let filename = if asset.language == "csharp" {
                    "c_sharp"
                } else {
                    &asset.language
                };
                if asset.source_path.as_deref()
                    != Some(format!("src/extraction/wasm/tree-sitter-{filename}.wasm").as_str())
                    || !asset.sha256.as_deref().is_some_and(valid_sha256)
                    || asset.bytes.is_none_or(|bytes| bytes < 8)
                {
                    return Err("CodeGraph 随仓资产身份无效".into());
                }
            }
            "tree_sitter_wasms_dependency" => {
                if !matches!(asset.language.as_str(), "objc" | "solidity")
                    || asset.source_path.is_some()
                    || asset.sha256.is_some()
                    || asset.bytes.is_some()
                {
                    return Err("CodeGraph 依赖资产身份无效".into());
                }
            }
            _ => return Err("CodeGraph 资产来源未知".into()),
        }
    }
    if vendored != 30 || seen != LANGUAGES.into_iter().collect() {
        return Err("CodeGraph grammar 覆盖集合不符".into());
    }
    Ok(())
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
