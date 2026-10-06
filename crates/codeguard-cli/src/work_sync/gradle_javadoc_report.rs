//! Gradle Javadoc报告的输入绑定与稳定问题投影；不授予可信关闭。
use super::{BlockerInput, FindingInput, ReportInput, safe_run_id};
use serde_json::{Value, json};
use std::path::Path;

pub(super) fn parse(
    root: &Path,
    workspace: &str,
    path: &Path,
    report: &Value,
    digest: String,
) -> Result<ReportInput, &'static str> {
    let keys = [
        "schema_version",
        "report_type",
        "workspace_binding",
        "workspace_id",
        "run_id",
        "checker_id",
        "authority",
        "coverage_proven",
        "delivery_decision",
        "inputs",
        "native",
        "findings",
        "blockers",
    ];
    if !report
        .as_object()
        .is_some_and(|o| o.len() == keys.len() && keys.iter().all(|k| o.contains_key(*k)))
        || report["schema_version"] != "0.1.0"
        || report["report_type"] != "gradle_javadoc_workbench_observation"
        || report["workspace_binding"] != "bound"
        || report["workspace_id"] != workspace
        || report["checker_id"] != "java.gradle.javadoc"
        || report["authority"] != "local_unverified"
        || report["coverage_proven"] != false
        || report["delivery_decision"] != "not_evaluated"
    {
        return Err("gradle_javadoc_report_identity_invalid");
    }
    let run = report["run_id"]
        .as_str()
        .filter(|r| safe_run_id(r) && r.starts_with("javadoc-gradle-"))
        .ok_or("gradle_javadoc_run_invalid")?;
    if path.file_stem().and_then(|s| s.to_str()) != Some(run) {
        return Err("gradle_javadoc_run_invalid");
    }
    let (findings, blockers) =
        crate::gradle_javadoc_workbench::project(root, &report["inputs"], &report["native"])?;
    if report["findings"] != json!(findings) || report["blockers"] != json!(blockers) {
        return Err("gradle_javadoc_projection_invalid");
    }
    let findings = findings
        .iter()
        .map(|f| {
            Ok(FindingInput {
                checker_id: "java.gradle.javadoc".into(),
                id: f["finding_id"]
                    .as_str()
                    .ok_or("gradle_finding_invalid")?
                    .into(),
                fingerprint: f["finding_fingerprint"]
                    .as_str()
                    .ok_or("gradle_finding_invalid")?
                    .into(),
                path: f["path"].as_str().ok_or("gradle_finding_invalid")?.into(),
                source_sha256: f["source_sha256"]
                    .as_str()
                    .ok_or("gradle_finding_invalid")?
                    .into(),
                rule_id: f["rule_id"]
                    .as_str()
                    .ok_or("gradle_finding_invalid")?
                    .into(),
                line: f["line"].as_u64().ok_or("gradle_finding_invalid")?,
            })
        })
        .collect::<Result<Vec<_>, &'static str>>()?;
    let blockers = blockers
        .iter()
        .map(|b| {
            Ok(BlockerInput {
                checker_id: "java.gradle.javadoc".into(),
                id: b["id"].as_str().ok_or("gradle_blocker_invalid")?.into(),
                fingerprint: b["fingerprint"]
                    .as_str()
                    .ok_or("gradle_blocker_invalid")?
                    .into(),
                reason: "gradle_javadoc_preparation_required".into(),
                diagnostic_reason: Some(
                    b["diagnostic_reason"]
                        .as_str()
                        .ok_or("gradle_blocker_invalid")?
                        .into(),
                ),
                build_root: b["build_root"]
                    .as_str()
                    .ok_or("gradle_blocker_invalid")?
                    .into(),
                scope: b["scope"].as_str().ok_or("gradle_blocker_invalid")?.into(),
                affected_paths: b["affected_paths"]
                    .as_array()
                    .ok_or("gradle_blocker_invalid")?
                    .iter()
                    .map(|p| {
                        p.as_str()
                            .map(str::to_owned)
                            .ok_or("gradle_blocker_invalid")
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            })
        })
        .collect::<Result<Vec<_>, &'static str>>()?;
    Ok(ReportInput {
        workspace_id: workspace.into(),
        run_id: run.into(),
        digest,
        findings,
        blockers,
        historical_findings: 0,
    })
}

/// 核对原任务、消费收据和当前输入后导入复检；失败仅保存观察，不导入源码问题。
pub(super) fn parse_recheck(
    root: &Path,
    workspace: &str,
    path: &Path,
    report: &Value,
    digest: String,
) -> Result<ReportInput, &'static str> {
    if !crate::gradle_javadoc_task_recheck::valid_shape(report)
        || report["workspace_id"] != workspace
        || report["run_id"].as_str() != path.file_stem().and_then(|p| p.to_str())
    {
        return Err("gradle_javadoc_recheck_identity_invalid");
    }
    let id = report["task_id"]
        .as_str()
        .ok_or("gradle_javadoc_task_invalid")?;
    let fact = codeguard_adapters::parse_unique_json(
        &codeguard_runtime::read_bounded_regular_file(
            &root.join(format!(".codeguard/findings/{id}/finding.json")),
            128 * 1024,
        )
        .map_err(|_| "gradle_javadoc_task_unavailable")?,
    )
    .map_err(|_| "gradle_javadoc_task_invalid")?;
    let finding = fact["kind"] == "finding";
    let brief = json!({"task_id":id,"checker_id":fact["checker_id"],"kind":fact["kind"],"scope":fact[if finding {"path"} else {"scope"}],"native_rule_id":if finding {fact["native_rule_id"].clone()}else{Value::Null},"evidence_ref":{"first_run_id":fact["first_run_id"],"first_report_sha256":fact["first_report_sha256"]}});
    if fact["id"] != id
        || fact["workspace_id"] != workspace
        || report["task_path"] != brief["scope"]
        || report["task_rule"] != brief["native_rule_id"]
        || report["origin"] != brief["evidence_ref"]
    {
        return Err("gradle_javadoc_recheck_task_binding_invalid");
    }
    let original = crate::gradle_javadoc_task_recheck::original(root, &brief)?;
    validate_bindings(&original, report)?;
    if report["task_input_stable"] == true
        && !crate::gradle_javadoc_task_recheck::inputs_current(root, report)
    {
        return Err("gradle_javadoc_recheck_inputs_changed");
    }
    // 无扫描和输入失效均不得再导入原生位置；保留失败收据供尝试记录使用。
    if report["scan"].is_null() || report["task_input_stable"] != true {
        return Ok(ReportInput {
            workspace_id: workspace.into(),
            run_id: report["run_id"]
                .as_str()
                .ok_or("gradle_javadoc_run_invalid")?
                .into(),
            digest,
            findings: Vec::new(),
            blockers: Vec::new(),
            historical_findings: 0,
        });
    }
    parse(root, workspace, path, &report["scan"], digest)
}

// 比较选定范围和配置/工具声明，不相信外层布尔字段；内层位置仍由project重算。
fn validate_bindings(original: &Value, report: &Value) -> Result<(), &'static str> {
    use std::collections::BTreeMap;
    let original_rows = original["inputs"]
        .as_array()
        .ok_or("gradle_javadoc_original_inputs_invalid")?;
    let old = original_rows
        .iter()
        .map(|r| {
            Ok((
                r["path"]
                    .as_str()
                    .ok_or("gradle_javadoc_original_inputs_invalid")?,
                r["sha256"]
                    .as_str()
                    .ok_or("gradle_javadoc_original_inputs_invalid")?,
            ))
        })
        .collect::<Result<BTreeMap<_, _>, &'static str>>()?;
    let rows = report["input_bindings"]
        .as_array()
        .ok_or("gradle_javadoc_bindings_invalid")?;
    let workspace = rows
        .iter()
        .filter(|r| r["location"] == "workspace")
        .map(|r| {
            Ok((
                r["path"]
                    .as_str()
                    .ok_or("gradle_javadoc_bindings_invalid")?,
                r["sha256"]
                    .as_str()
                    .ok_or("gradle_javadoc_bindings_invalid")?,
            ))
        })
        .collect::<Result<BTreeMap<_, _>, &'static str>>()?;
    let scope = !workspace.is_empty() && workspace.keys().eq(old.keys());
    let configuration = scope
        && old
            .iter()
            .filter(|(p, _)| !p.ends_with(".java"))
            .all(|(p, s)| workspace.get(p) == Some(s));
    if workspace.len() != rows.iter().filter(|r| r["location"] == "workspace").count()
        || report["source_scope_matches"] != scope
        || report["configuration_matches"] != configuration
        || (!workspace.is_empty() && !scope)
    {
        return Err("gradle_javadoc_scope_or_configuration_invalid");
    }
    let bundles = rows
        .iter()
        .filter(|r| r["location"] == "bundle")
        .collect::<Vec<_>>();
    let tools = rows
        .iter()
        .filter(|r| r["location"] == "tool")
        .collect::<Vec<_>>();
    let java = tools
        .iter()
        .find(|r| r["path"].as_str().is_some_and(|p| p.ends_with("/bin/java")));
    let release = tools
        .iter()
        .find(|r| r["path"].as_str().is_some_and(|p| p.ends_with("/release")));
    let complete = bundles.len() == 1 && tools.len() == 2 && java.is_some() && release.is_some();
    if !complete && (!bundles.is_empty() || !tools.is_empty()) {
        return Err("gradle_javadoc_tool_bindings_invalid");
    }
    let keys = [
        "gradle_bundle_sha256",
        "java_entry_sha256",
        "jdk_release_sha256",
    ];
    let identity_complete = complete && keys.iter().all(|k| original["native"][*k].is_string());
    let current = if complete {
        vec![
            &bundles[0]["sha256"],
            &java.unwrap()["sha256"],
            &release.unwrap()["sha256"],
        ]
    } else {
        vec![]
    };
    let matches = complete
        && keys
            .iter()
            .zip(&current)
            .all(|(k, s)| original["native"][*k].is_null() || original["native"][*k] == **s);
    if report["original_tool_identity_complete"] != identity_complete
        || report["tool_identity_matches"] != matches
    {
        return Err("gradle_javadoc_tool_claim_invalid");
    }
    if !report["scan"].is_null() {
        let preparation_only = !complete
            && report["scan"]["native"] == crate::gradle_javadoc_probe::missing_prerequisites();
        if !scope || !configuration || (!matches && !preparation_only) {
            return Err("gradle_javadoc_native_context_invalid");
        }
        let scan_rows = report["scan"]["inputs"]
            .as_array()
            .ok_or("gradle_javadoc_scan_inputs_invalid")?;
        let inner = scan_rows
            .iter()
            .map(|r| {
                Ok((
                    r["path"]
                        .as_str()
                        .ok_or("gradle_javadoc_scan_inputs_invalid")?,
                    r["sha256"]
                        .as_str()
                        .ok_or("gradle_javadoc_scan_inputs_invalid")?,
                ))
            })
            .collect::<Result<BTreeMap<_, _>, &'static str>>()?;
        if inner != workspace || inner.len() != scan_rows.len() {
            return Err("gradle_javadoc_scan_inputs_invalid");
        }
        for (key, sha) in keys.iter().zip(&current) {
            if !report["scan"]["native"][*key].is_null() && report["scan"]["native"][*key] != **sha
            {
                return Err("gradle_javadoc_scan_tools_invalid");
            }
        }
        if report["scan"]["native"]["init_scripts_sha256"]
            != original["native"]["init_scripts_sha256"]
        {
            return Err("gradle_javadoc_scan_scripts_invalid");
        }
    }
    Ok(())
}
