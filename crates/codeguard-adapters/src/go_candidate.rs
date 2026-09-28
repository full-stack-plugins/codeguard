//! Go 六类别原生工具候选档案；所有槽位保持 gap，不能作为执行或门禁授权。

use codeguard_core::{CANDIDATE_PLATFORMS, CHECK_CATEGORIES};
use serde::Deserialize;
use std::collections::BTreeSet;

/// Go 工具候选及已知覆盖缺口，不代表该工具已安装或适配器已验收。
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoCandidateSlot {
    /// CodeGuard 的六类别之一。
    pub category: String,
    /// 适用性；缺工具不能写成不适用。
    pub applicability: String,
    /// 目前只能是 gap。
    pub status: String,
    /// 候选原生工具的稳定名称。
    pub candidate_tool: String,
    /// 固定候选发行版或运行时解析的项目 Go 工具链。
    pub candidate_version: String,
    /// 版本取证状态。
    pub version_status: String,
    /// 候选字面命令，仅供设计和后续验证；本模块绝不执行。
    pub candidate_argv: Vec<String>,
    /// CVE 数据库新鲜度是否属于必备证据。
    pub requires_database_freshness: bool,
    /// 原生工具或语言规范的一手来源。
    pub sources: Vec<String>,
    /// 尚未解决的适配、归属或覆盖缺口。
    pub coverage_gaps: Vec<String>,
}

/// Go Modules 方言及候选平台的六类别完整档案。
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoCandidateProfile {
    /// 档案协议版本。
    pub schema_version: String,
    /// 规范语言 ID。
    pub language: String,
    /// 当前考虑的依赖/构建方言。
    pub dialect: String,
    /// 需要分别验收的平台，列表本身不证明支持。
    pub platforms: Vec<String>,
    /// 跨平台验证状态。
    pub platform_validation: String,
    /// 六类别候选槽位。
    pub categories: Vec<GoCandidateSlot>,
}

/// 解析内置 Go 候选档案并执行与外部输入相同的严格校验。
pub fn bundled_go_candidate_profile() -> Result<GoCandidateProfile, String> {
    parse_go_candidate_profile(include_str!(
        "../../../rulepacks/go_static_candidate_v1.json"
    ))
}

/// 拒绝缺槽、重复、虚报实现或不受信任的候选工具定义。
pub fn parse_go_candidate_profile(raw: &str) -> Result<GoCandidateProfile, String> {
    let profile: GoCandidateProfile =
        serde_json::from_str(raw).map_err(|error| error.to_string())?;
    if profile.schema_version != "1.0.0"
        || profile.language != "go"
        || profile.dialect != "go_modules"
        || profile.platform_validation != "unverified"
        || profile.platforms.len() != CANDIDATE_PLATFORMS.len()
        || profile
            .platforms
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>()
            != CANDIDATE_PLATFORMS.into_iter().collect()
    {
        return Err("Go 候选档案的版本、方言或平台集合无效".into());
    }
    let categories: BTreeSet<&str> = profile
        .categories
        .iter()
        .map(|slot| slot.category.as_str())
        .collect();
    if profile.categories.len() != CHECK_CATEGORIES.len()
        || categories != CHECK_CATEGORIES.into_iter().collect()
    {
        return Err("Go 候选档案必须恰有六个不重复类别".into());
    }
    for slot in &profile.categories {
        let (expected_tool, expected_version, expected_argv): (&str, &str, &[&str]) =
            match slot.category.as_str() {
                "lint" => ("go_vet", "project_toolchain", &["go", "vet", "./..."]),
                "comments" => (
                    "staticcheck",
                    "2026.1",
                    &[
                        "staticcheck",
                        "-checks=ST1000,ST1020,ST1021,ST1022",
                        "./...",
                    ],
                ),
                "dependencies" => (
                    "go_list_modules",
                    "project_toolchain",
                    &["go", "list", "-m", "-json", "all"],
                ),
                "cve" => ("govulncheck", "v1.1.4", &["govulncheck", "-json", "./..."]),
                "security" => ("gosec", "v2.28.0", &["gosec", "-fmt=json", "./..."]),
                "build" => ("go_build", "project_toolchain", &["go", "build", "./..."]),
                _ => return Err("Go 候选类别未知".into()),
            };
        if slot.applicability != "applicable" || slot.status != "gap" {
            return Err(format!(
                "Go {} 尚未验收，不能声明不适用或已实现",
                slot.category
            ));
        }
        if slot.candidate_tool.is_empty()
            || slot.candidate_argv.is_empty()
            || slot.candidate_argv.iter().any(|arg| arg.is_empty())
            || slot.sources.is_empty()
            || slot
                .sources
                .iter()
                .any(|source| !source.starts_with("https://") || source.contains(' '))
            || slot.coverage_gaps.is_empty()
            || slot.coverage_gaps.iter().any(String::is_empty)
        {
            return Err(format!("Go {} 缺少原生候选或缺口依据", slot.category));
        }
        if slot.candidate_tool != expected_tool
            || slot.candidate_version != expected_version
            || slot
                .candidate_argv
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                != expected_argv
        {
            return Err(format!("Go {} 候选工具与类别不符", slot.category));
        }
        if slot.candidate_version == "latest"
            || slot.candidate_version.is_empty()
            || !matches!(
                slot.version_status.as_str(),
                "resolved_at_run" | "release_candidate_unvalidated"
            )
            || ((slot.candidate_version == "project_toolchain")
                != (slot.version_status == "resolved_at_run"))
        {
            return Err(format!(
                "Go {} 候选版本未固定或未标明解析方式",
                slot.category
            ));
        }
        if slot.category == "comments" && slot.candidate_argv[0] == "gofmt" {
            return Err("gofmt 不能作为注释规范检测器".into());
        }
        if (slot.category == "cve") != slot.requires_database_freshness {
            return Err(format!("Go {} 漏洞库新鲜度声明不一致", slot.category));
        }
    }
    Ok(profile)
}
