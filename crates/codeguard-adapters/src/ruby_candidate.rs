//! Ruby 原生六类别研究档案的严格读者；不启动原生进程，也不注册已验收能力。
use crate::{parse_unique_json, ruby_candidate_profile::RubyCandidateProfile};
use codeguard_core::{CANDIDATE_PLATFORMS, CHECK_CATEGORIES};
use std::collections::BTreeSet;

/// 读取内置 Ruby 档案并核对六类别、工具对应、方言、版本和条件；返回研究档案或错误。
pub fn bundled_ruby_candidate_profile() -> Result<RubyCandidateProfile, String> {
    parse_ruby_candidate_profile(include_str!(
        "../../../rulepacks/ruby_static_candidate_v1.json"
    ))
}
/// 消费外部候选 JSON；最多 64KiB，拒绝重复键、未知字段、类别缺项及虚报能力。
pub fn parse_ruby_candidate_profile(raw: &str) -> Result<RubyCandidateProfile, String> {
    if raw.len() > 64 * 1024 {
        return Err("Ruby 候选档案超出字节预算".into());
    }
    let value = parse_unique_json(raw.as_bytes()).map_err(str::to_string)?;
    let profile: RubyCandidateProfile = serde_json::from_value(value).map_err(|e| e.to_string())?;
    if profile.schema_version != "1.0.0"
        || profile.language != "ruby"
        || profile.runtime_validation != "unverified"
        || profile.platform_validation != "unverified"
        || !exact_set(&profile.runtime_dialects, &["mri", "jruby", "truffleruby"])
        || !exact_set(&profile.platforms, &CANDIDATE_PLATFORMS)
        || !exact_set(
            &profile.target_version_inputs,
            &[
                ".ruby-version",
                ".tool-versions",
                "mise.toml",
                "Gemfile",
                "Gemfile.lock",
                "*.gemspec",
                ".rubocop.yml",
            ],
        )
        || !valid_gaps(&profile.version_gaps)
    {
        return Err("Ruby 档案版本、方言、版本输入或平台集合未验证/缺失".into());
    }
    let names: Vec<String> = profile
        .categories
        .iter()
        .map(|s| s.category.clone())
        .collect();
    if !exact_set(&names, &CHECK_CATEGORIES) {
        return Err("Ruby 档案必须恰有六个不重复类别".into());
    }
    for slot in &profile.categories {
        let expected: &[&str] = match slot.category.as_str() {
            "lint" => &["rubocop"],
            "comments" => &["rubocop_documentation", "rubocop_documentation_method"],
            "dependencies" => &["bundle_list"],
            "cve" => &["bundler_audit"],
            "security" => &["rubocop_security", "brakeman"],
            "build" => &["gem_build"],
            _ => return Err("Ruby 类别未知".into()),
        };
        let ids: Vec<String> = slot.tools.iter().map(|t| t.tool_id.clone()).collect();
        if slot.status != "gap"
            || !exact_set(&ids, expected)
            || slot.applicability
                != if slot.category == "build" {
                    "project_dependent"
                } else {
                    "applicable"
                }
            || slot.requires_database_freshness != (slot.category == "cve")
            || !valid_gaps(&slot.coverage_gaps)
        {
            return Err(format!(
                "Ruby {} 的类别适用性、工具或缺口无效",
                slot.category
            ));
        }
        for tool in &slot.tools {
            let (argv, scope): (&[&str], &str) = match tool.tool_id.as_str() {
                "rubocop" => (
                    &["bundle", "exec", "rubocop", "--format", "json"],
                    "ruby_project",
                ),
                "rubocop_documentation" => (
                    &[
                        "bundle",
                        "exec",
                        "rubocop",
                        "--only",
                        "Style/Documentation",
                        "--format",
                        "json",
                    ],
                    "ruby_project",
                ),
                "rubocop_documentation_method" => (
                    &[
                        "bundle",
                        "exec",
                        "rubocop",
                        "--only",
                        "Style/DocumentationMethod",
                        "--format",
                        "json",
                    ],
                    "ruby_project",
                ),
                "bundle_list" => (&["bundle", "list"], "bundler_project"),
                "bundler_audit" => (
                    &["bundle-audit", "check", "--no-update", "--format", "json"],
                    "bundler_project",
                ),
                "rubocop_security" => (
                    &[
                        "bundle", "exec", "rubocop", "--only", "Security", "--format", "json",
                    ],
                    "ruby_project",
                ),
                "brakeman" => (
                    &[
                        "bundle",
                        "exec",
                        "brakeman",
                        "--format",
                        "json",
                        "--no-pager",
                    ],
                    "rails_project",
                ),
                "gem_build" => (&["gem", "build", "SELECTED_GEMSPEC"], "gem_project"),
                _ => return Err("Ruby 工具未知".into()),
            };
            if tool.candidate_version != "project_locked"
                || tool.version_status != "unvalidated_project_lock_required"
                || tool.scope != scope
                || tool
                    .candidate_argv
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>()
                    != argv
                || tool.sources.is_empty()
                || tool.sources.len() > 8
                || tool.sources.iter().any(|s| {
                    !s.starts_with("https://")
                        || s.len() > 512
                        || s.bytes().any(|b| b.is_ascii_whitespace())
                })
            {
                return Err(format!(
                    "Ruby {} 的版本、命令、范围或来源无效",
                    tool.tool_id
                ));
            }
        }
    }
    Ok(profile)
}
fn exact_set(actual: &[String], expected: &[&str]) -> bool {
    actual.len() == expected.len()
        && actual.iter().map(String::as_str).collect::<BTreeSet<_>>()
            == expected.iter().copied().collect()
}
fn valid_gaps(gaps: &[String]) -> bool {
    !gaps.is_empty()
        && gaps.len() <= 32
        && gaps.iter().collect::<BTreeSet<_>>().len() == gaps.len()
        && gaps.iter().all(|s| {
            !s.is_empty()
                && s.len() <= 256
                && s.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        })
}
