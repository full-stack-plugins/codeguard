//! 已记录的npm完整性问题只恢复本地准备范围，不授权检查或关闭。
use codeguard_runtime::read_bounded_regular_file;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::Path};

/// 参数为物理工作区；返回当前清单不可用的已记录范围，或记录不可核验原因。
pub(crate) fn collect(root: &Path) -> Result<Vec<String>, &'static str> {
    let baseline = crate::workspace_refresh::read_workspace_baseline(root)
        .map_err(|_| "recorded_npm_workspace_invalid")?;
    let Some(workspace_id) = baseline.as_ref().and_then(|b| b.workspace_id()) else {
        return Ok(Vec::new());
    };
    let findings = root.join(".codeguard/findings");
    match fs::symlink_metadata(&findings) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        _ if findings.canonicalize().ok().as_ref() != Some(&findings) || !findings.is_dir() => {
            return Err("recorded_npm_facts_unavailable");
        }
        _ => {}
    }
    let mut roots = BTreeSet::new();
    for (index, entry) in fs::read_dir(&findings)
        .map_err(|_| "recorded_npm_facts_unavailable")?
        .enumerate()
    {
        if index >= 10_000 {
            return Err("recorded_npm_facts_limit");
        }
        let entry = entry.map_err(|_| "recorded_npm_facts_unavailable")?;
        let name = entry.file_name();
        let id = name.to_str().ok_or("recorded_npm_fact_invalid")?;
        // 普通finding与其它非阻塞记录不提供npm范围。
        if !id.starts_with("CG-B-") {
            continue;
        }
        let directory = entry.path();
        if directory.canonicalize().ok().as_ref() != Some(&directory) || !directory.is_dir() {
            return Err("recorded_npm_fact_path_invalid");
        }
        let path = directory.join("finding.json");
        if path.canonicalize().ok().as_ref() != Some(&path) {
            return Err("recorded_npm_fact_path_invalid");
        }
        let bytes = read_bounded_regular_file(&path, 128 * 1024)
            .map_err(|_| "recorded_npm_fact_unavailable")?;
        let fact = codeguard_adapters::parse_unique_json(&bytes)
            .map_err(|_| "recorded_npm_fact_invalid")?;
        if fact["checker_id"] != "node.npm.audit" {
            continue;
        }
        let build = validate(&fact, id, workspace_id)?;
        let project = if build == "." {
            root.to_path_buf()
        } else {
            root.join(build)
        };
        if project.canonicalize().ok().as_ref() != Some(&project) || !project.is_dir() {
            return Err("recorded_npm_project_unavailable");
        }
        if crate::npm_input_state::observe(&project.join("package.json"), 256 * 1024).0 != "present"
        {
            roots.insert(build.to_owned());
        }
    }
    Ok(roots.into_iter().collect())
}

fn validate<'a>(fact: &'a Value, id: &str, workspace_id: &str) -> Result<&'a str, &'static str> {
    let fields = [
        "schema_version",
        "kind",
        "id",
        "workspace_id",
        "fingerprint",
        "checker_id",
        "reason_code",
        "build_root",
        "scope",
        "first_affected_paths",
        "first_run_id",
        "first_report_sha256",
        "state",
        "authority",
        "delivery_decision",
    ];
    let object = fact.as_object().ok_or("recorded_npm_fact_invalid")?;
    if !fields.iter().all(|field| object.contains_key(*field))
        || object
            .keys()
            .any(|field| !fields.contains(&field.as_str()) && field != "first_diagnostic_reason")
        || fact["schema_version"] != "0.1.0"
        || fact["kind"] != "blocker"
        || fact["id"] != id
        || fact["workspace_id"] != workspace_id
        || fact["state"] != "open"
        || fact["authority"] != "local_unverified"
        || fact["delivery_decision"] != "not_evaluated"
        || fact["reason_code"] != "npm_audit_coverage_unverified"
        || !fact["first_report_sha256"].as_str().is_some_and(valid_hash)
        || !fact["first_run_id"].as_str().is_some_and(|s| {
            !s.is_empty()
                && s.len() <= 120
                && s.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
        })
        || object.get("first_diagnostic_reason").is_some_and(|v| {
            !v.as_str().is_some_and(|s| {
                !s.is_empty()
                    && s.len() <= 120
                    && s.bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
            })
        })
    {
        return Err("recorded_npm_fact_invalid");
    }
    let build = fact["build_root"]
        .as_str()
        .ok_or("recorded_npm_fact_invalid")?;
    if build != "."
        && (build.is_empty()
            || build.contains('\\')
            || build.contains(':')
            || build.chars().any(char::is_control)
            || build
                .split('/')
                .any(|s| s.is_empty() || s == "." || s == ".."))
    {
        return Err("recorded_npm_scope_invalid");
    }
    let mut hash = Sha256::new();
    for part in [
        "codeguard-blocker-v1",
        "node.npm.audit",
        "npm_audit_coverage_unverified",
        build,
        build,
    ] {
        hash.update((part.len() as u64).to_be_bytes());
        hash.update(part.as_bytes());
    }
    let fingerprint = format!("{:x}", hash.finalize());
    let affected: Vec<String> = ["package.json", "package-lock.json"]
        .into_iter()
        .map(|file| {
            if build == "." {
                file.into()
            } else {
                format!("{build}/{file}")
            }
        })
        .collect();
    if fact["fingerprint"] != fingerprint
        || id != format!("CG-B-{}", &fingerprint[..32])
        || fact["scope"] != build
        || fact["first_affected_paths"] != serde_json::json!(affected)
    {
        return Err("recorded_npm_identity_invalid");
    }
    Ok(build)
}
fn valid_hash(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
