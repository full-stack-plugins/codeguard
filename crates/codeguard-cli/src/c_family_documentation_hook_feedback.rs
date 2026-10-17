//! C/C++文档原任务复检的有界Hook投影；不授予可信关闭。
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{path::Path, time::Instant};

/// 核验原任务报告及当前输入，返回脱敏局部摘要；根/任务/复检/截止来自执行器。
pub(crate) fn project(
    root: &Path,
    brief: &Value,
    report: &Value,
    deadline: Instant,
) -> Result<Value, &'static str> {
    let scan = &report["native_scan"];
    let structure = matches!(
        brief["checker_id"].as_str(),
        Some("c.clang.documentation_structure" | "cpp.clang.documentation_structure")
    );
    let valid = if structure {
        report["schema_version"] == "0.37.0"
            && crate::c_family_structure_task_recheck::valid_shape(root, scan)
    } else {
        report["schema_version"] == "0.36.0"
            && crate::c_family_comments_task_recheck::valid_shape(root, scan)
    };
    if !valid || scan["task_id"] != brief["task_id"] {
        return Err("clang_documentation_hook_report_invalid");
    }
    let mut current = Instant::now() < deadline
        && !codeguard_runtime::sigint_cancellation_requested()
        && scan["input_stable"] == true
        && if structure {
            crate::c_family_structure_task_recheck::inputs_current(root, scan)
        } else {
            crate::c_family_comments_task_recheck::inputs_current(root, scan)
        };
    let mut positions = Vec::new();
    let mut total = 0;
    if current && scan["local_scan_complete"] == true {
        let rows = if structure {
            crate::work_sync::c_family_structure_report::positions(
                &crate::c_family_structure_task_recheck::normal(scan),
            )
        } else {
            scan["native"]["diagnostics"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|d| d["rule_id"] == brief["native_rule_id"])
                .cloned()
                .collect()
        };
        total = rows.len();
        for row in rows.iter().take(8) {
            positions.push(json!({"line":row["line"],"column_byte":row["column_byte"],"rule_id":if structure{json!(crate::work_sync::c_family_structure_report::RULE)}else{brief["native_rule_id"].clone()},"missing_component_count":if structure{row["missing_components"].as_array().map_or(0,Vec::len)}else{0}}));
        }
    }
    let reference = if current && report["event_persisted"] == true {
        persisted(root, scan)
    } else {
        None
    };
    if report["event_persisted"] == true && reference.is_none() {
        current = false;
        positions.clear();
        total = 0;
    }
    // 写入事件不等于报告可引用；明确单独反馈收据状态，不能伪造可关闭证据。
    Ok(
        json!({"schema_version":"0.9.0","documentation_input_current":current,"documentation_rule_source":if structure{"codeguard_structural_policy"}else{"native_clang_warning"},"documentation_standard":scan["standard"],"documentation_source_sha256":scan["source_sha256"],"documentation_positions":positions,"documentation_position_count":total,"documentation_positions_truncated":total>8,"documentation_report_ref":reference,"documentation_reference_status":if reference.is_some(){"verified_consumed"}else{"unavailable"},"detailed_contract_qualification":"not_granted","task_closure":"not_permitted"}),
    )
}
fn persisted(root: &Path, scan: &Value) -> Option<Value> {
    let run = scan["run_id"].as_str()?;
    let reference = format!(".codeguard/reports/{run}.json");
    let bytes =
        codeguard_runtime::read_bounded_regular_file(&root.join(&reference), 16 * 1024 * 1024)
            .ok()?;
    if codeguard_adapters::parse_unique_json(&bytes).ok().as_ref() != Some(scan) {
        return None;
    }
    let sha = format!("{:x}", Sha256::digest(&bytes));
    let marker = codeguard_runtime::read_bounded_regular_file(
        &root.join(format!(".codeguard/state/consumed/{run}.json")),
        4096,
    )
    .ok()?;
    let expected=serde_json::to_vec_pretty(&json!({"schema_version":"0.1.0","workspace_id":scan["workspace_id"],"run_id":run,"report_sha256":sha})).ok()?;
    if marker != expected {
        return None;
    }
    Some(json!({"run_id":run,"report_ref":reference,"report_sha256":sha}))
}

#[cfg(test)]
mod tests {
    use super::project;
    use serde_json::Value;
    use std::{
        fs,
        path::PathBuf,
        process::Command,
        time::{Duration, Instant, SystemTime, UNIX_EPOCH},
    };

    #[test]
    #[ignore = "requires explicit current CLI and installed Clang"]
    fn changed_input_and_forged_receipts_never_supply_current_report_authority() {
        let cli = PathBuf::from(std::env::var_os("CODEGUARD_RECHECK_CLI").expect("current CLI"));
        let tool = PathBuf::from(std::env::var_os("CODEGUARD_CLANG_BIN").expect("installed Clang"));
        let root = std::env::temp_dir().join(format!(
            "cg-doc-hook-project-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let root = root.canonicalize().unwrap();
        struct Cleanup(PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }
        let _cleanup = Cleanup(root.clone());
        let source = root.join("api.c");
        let text = "int f(int x) { return x; }\n";
        fs::write(&source, text).unwrap();
        assert_eq!(
            Command::new(&cli)
                .arg("init")
                .arg(&root)
                .args(["--apply", "--format=json"])
                .output()
                .unwrap()
                .status
                .code(),
            Some(3)
        );
        let first = Command::new(&cli)
            .args(["comments", "c"])
            .arg(&source)
            .arg("--clang-tool")
            .arg(&tool)
            .args(["--standard", "c11", "--format=json"])
            .output()
            .unwrap();
        let first: Value = serde_json::from_slice(&first.stdout).unwrap();
        let id = first["structural_workbench"]["task_ids"][0]
            .as_str()
            .unwrap();
        let view = crate::next_command::read_task_brief(&root, id).unwrap();
        let output = Command::new(&cli)
            .args(["task", "verify", id])
            .arg(&root)
            .arg("--clang-tool")
            .arg(&tool)
            .arg("--format=json")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(3));
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        let deadline = Instant::now() + Duration::from_secs(30);
        let valid = project(&root, &view, &report, deadline).unwrap();
        assert_eq!(valid["documentation_input_current"], true);
        assert!(valid["documentation_report_ref"].is_object());
        fs::write(&source, "int changed(int x) { return x + 1; }\n").unwrap();
        let stale = project(&root, &view, &report, deadline).unwrap();
        assert_eq!(stale["documentation_input_current"], false);
        assert_eq!(stale["documentation_positions"], serde_json::json!([]));
        assert_eq!(stale["documentation_report_ref"], Value::Null);
        fs::write(&source, text).unwrap();
        let scan = &report["native_scan"];
        let receipt = root.join(format!(
            ".codeguard/state/consumed/{}.json",
            scan["run_id"].as_str().unwrap()
        ));
        let bytes = fs::read(&receipt).unwrap();
        fs::write(&receipt, b"{}").unwrap();
        let forged = project(&root, &view, &report, deadline).unwrap();
        assert_eq!(forged["documentation_input_current"], false);
        assert_eq!(forged["documentation_positions"], serde_json::json!([]));
        assert_eq!(forged["documentation_reference_status"], "unavailable");
        assert_eq!(forged["documentation_report_ref"], Value::Null);
        fs::write(receipt, bytes).unwrap();
        let expired = project(&root, &view, &report, Instant::now()).unwrap();
        assert_eq!(expired["documentation_input_current"], false);
        assert_eq!(expired["documentation_position_count"], 0);
        let mut wrong = report.clone();
        wrong["native_scan"]["task_id"] = serde_json::json!("CG-00000000000000000000000000000000");
        assert!(project(&root, &view, &wrong, deadline).is_err());
    }
}
