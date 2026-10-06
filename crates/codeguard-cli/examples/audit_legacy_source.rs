//! 固定旧源码差异的只读审计；不运行旧检查器，不签发质量结论。
use codeguard_adapters::parse_unique_json;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::Path;
use std::process::{Command, ExitCode};

fn git(root: &Path, args: &[&str]) -> Result<Option<Vec<u8>>, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Ok(None);
    }
    if output.stdout.len() > 16 * 1024 * 1024 {
        return Err("固定旧源码超过审计预算".into());
    }
    Ok(Some(output.stdout))
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn verify(root: &Path, report: &Value) -> Result<(), String> {
    if report["report_type"] != "legacy_source_migration_audit"
        || report["schema_version"] != "0.1.0"
        || report["delivery_decision"] != "not_evaluated"
    {
        return Err("不支持的旧源码审计协议".into());
    }
    let base = report["base_commit"].as_str().ok_or("缺基线身份")?;
    let head = report["source_commit"].as_str().ok_or("缺源码身份")?;
    let rows = report["rows"].as_array().ok_or("缺差异清单")?;
    let changes = git(
        root,
        &[
            "diff",
            "--name-only",
            base,
            head,
            "--",
            "bin",
            "scripts",
            "hooks",
        ],
    )?
    .ok_or("无法读取固定旧源码差异")?;
    let changes = std::str::from_utf8(&changes).map_err(|e| e.to_string())?;
    let expected: BTreeSet<&str> = changes.lines().collect();
    let listed: BTreeSet<&str> = rows.iter().filter_map(|r| r["path"].as_str()).collect();
    if expected != listed || listed.len() != rows.len() || report["file_count"] != rows.len() {
        return Err("遗漏、追加或重复旧源码差异".into());
    }
    for row in rows {
        let path = row["path"].as_str().ok_or("缺源码路径")?;
        if !matches!(
            row["disposition"].as_str(),
            Some("preserve" | "correct" | "legacy")
        ) || row["rationale"].as_str().is_none_or(str::is_empty)
        {
            return Err(format!("未分类旧源码：{path}"));
        }
        let spec = row["spec_ref"].as_str().ok_or("缺规格引用")?;
        if !spec.starts_with("openspec/changes/introduce-rust-codeguard-cli/specs/")
            || !Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join(spec)
                .is_file()
        {
            return Err("规格引用不存在".into());
        }
        for (commit, field) in [(base, "before_sha256"), (head, "after_sha256")] {
            let bytes = git(root, &["show", &format!("{commit}:{path}")])?;
            let actual = bytes.as_deref().map(digest);
            if row[field].as_str() != actual.as_deref() {
                return Err(format!("旧源码摘要失配：{path}/{field}"));
            }
        }
        let fixture = row["fixture_ref"].as_str().ok_or("缺夹具引用")?;
        let bytes = git(root, &["show", &format!("{head}:{fixture}")])?.ok_or("固定夹具不存在")?;
        if row["fixture_sha256"].as_str() != Some(digest(&bytes).as_str()) {
            return Err(format!("旧源码夹具失配：{fixture}"));
        }
    }
    Ok(())
}
fn report() -> Result<Value, String> {
    parse_unique_json(include_bytes!(
        "../../../tests/acceptance/evidence/legacy-source-audit-2026-10-06.json"
    ))
    .map_err(str::to_owned)
}
fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if !matches!(args.as_slice(), [option, _] if option == "--check-source") {
        eprintln!("用法：audit_legacy_source --check-source PLUGIN_GIT_ROOT");
        return ExitCode::from(2);
    }
    match report().and_then(|r| verify(Path::new(&args[1]), &r)) {
        Ok(()) => {
            println!("21项旧源码差异、分类、规格和固定夹具摘要一致；交付未评估");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(3)
        }
    }
}
#[cfg(test)]
mod tests {
    use super::{report, verify};
    use std::path::Path;
    #[test]
    fn frozen_source_audit_does_not_pass_without_a_repository() {
        let report = report().unwrap();
        assert_eq!(report["file_count"], 21);
        assert_eq!(report["rows"].as_array().unwrap().len(), 21);
        // 无仓库或引用失效是未完成，不能仅因表格存在判审计通过。
        assert!(verify(Path::new("/nonexistent-codeguard-audit-root"), &report).is_err());
    }
    #[test]
    #[ignore = "需完整固定插件Git源码；显式源码审计验收运行"]
    fn fixed_source_and_every_mutation_are_checked_against_actual_git() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../codeguard-plugin");
        let baseline = report().unwrap();
        verify(&root, &baseline).unwrap();
        for variant in 0..6 {
            let mut candidate = baseline.clone();
            match variant {
                0 => {
                    candidate["rows"].as_array_mut().unwrap().pop();
                }
                1 => {
                    let row = candidate["rows"][0].clone();
                    candidate["rows"].as_array_mut().unwrap().push(row);
                }
                2 => candidate["rows"][0]["after_sha256"] = "0".repeat(64).into(),
                3 => candidate["rows"][0]["fixture_sha256"] = "0".repeat(64).into(),
                4 => candidate["rows"][0]["disposition"] = "unclassified".into(),
                _ => candidate["rows"][0]["spec_ref"] = "missing-spec".into(),
            }
            assert!(verify(&root, &candidate).is_err(), "variant {variant}");
        }
    }
}
