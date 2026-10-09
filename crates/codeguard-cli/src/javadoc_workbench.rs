//! 将 Javadoc 局部观察接入持久修复队列，不授予交付与自动关闭资格。
use crate::next_command::read_local_brief_for_checker;
use crate::work_sync::{save_local_report, sync_local_workspace};
use crate::workspace_refresh::read_workspace_baseline;
use codeguard_runtime::read_bounded_regular_file;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// 对已初始化项目保存当前局部观察；错误保持可见，不创建工作区或关闭问题。
pub(crate) fn connect(root: &Path, feedback: &Value) -> Value {
    let result = prepare(root, feedback).and_then(|report| {
        save_local_report(root, &report)?;
        let summary = sync_local_workspace(root)?;
        Ok(json!({"status":if summary.failed_reports == 0 {"synced_partial"} else {"sync_incomplete"},"new_findings":summary.new_findings,"new_blockers":summary.new_blockers,"next":read_local_brief_for_checker(root,"java.jdk.javadoc").unwrap_or_else(|reason| json!({"disposition":"verification_required","reason":reason})),"task_verify_status":"local_observation_only"}))
    });
    result.unwrap_or_else(|reason| json!({"status":reason,"new_findings":0,"new_blockers":0,"next":null,"task_verify_status":"local_observation_only"}))
}

pub(crate) fn prepare(root: &Path, feedback: &Value) -> Result<Value, &'static str> {
    let baseline = read_workspace_baseline(root).map_err(|_| "workspace_invalid")?;
    let id = baseline
        .as_ref()
        .and_then(|b| b.workspace_id())
        .ok_or("workspace_not_initialized")?;
    let native = &feedback["native_observation"];
    let explicit = native["report_type"] == "java_javadoc_local_feedback";
    let rows = if explicit {
        let source = Path::new(native["path"].as_str().ok_or("javadoc_path_invalid")?)
            .canonicalize()
            .map_err(|_| "javadoc_source_unavailable")?;
        let relative = source
            .strip_prefix(root)
            .map_err(|_| "source_outside_workspace")?
            .to_str()
            .ok_or("javadoc_path_invalid")?;
        if !relative.ends_with(".java") {
            return Err("javadoc_path_invalid");
        }
        vec![
            json!({"path":relative,"reason":native["reason"],"configuration_ref":null,"configuration_sha256":null,"observation":native}),
        ]
    } else {
        if native["report_type"] != "java_javadoc_project_probe"
            || native["probe_mode"] != "jdk_single_file"
        {
            return Err("javadoc_workbench_scope_not_integrated");
        }
        native["files"]
            .as_array()
            .filter(|r| r.len() <= 100_000)
            .ok_or("javadoc_observations_invalid")?
            .clone()
    };
    let mut sources = Vec::new();
    let mut seen_paths = BTreeSet::new();
    let mut total_bytes = 0_usize;
    for row in rows {
        let relative = row["path"].as_str().ok_or("javadoc_path_invalid")?;
        if !seen_paths.insert(relative.to_owned()) {
            return Err("javadoc_source_duplicate");
        }
        let bytes = read_bounded_regular_file(&root.join(relative), 16 * 1024 * 1024)
            .map_err(|_| "javadoc_source_unavailable")?;
        // 与 Maven 工作台相同的总量字节预算；超限保持整体拒绝而非裁剪输入。
        total_bytes = total_bytes
            .checked_add(bytes.len())
            .filter(|total| *total <= 128 * 1024 * 1024)
            .ok_or("javadoc_sources_byte_budget_exceeded")?;
        let reason = row["reason"].as_str().ok_or("javadoc_reason_invalid")?;
        let mut findings = Vec::new();
        let observation = &row["observation"];
        if !observation.is_null() {
            if observation["schema_version"] != "0.2.0" {
                return Err("javadoc_native_protocol_invalid");
            }
            if !observation["source_sha256"].is_null()
                && observation["source_sha256"] != format!("{:x}", Sha256::digest(&bytes))
            {
                return Err("javadoc_source_changed");
            }
            if matches!(
                observation["local_status"].as_str(),
                Some("findings_observed_untrusted" | "clean_scope_unproven")
            ) {
                let mut occurrences = BTreeMap::new();
                for native in observation["findings"]
                    .as_array()
                    .ok_or("javadoc_findings_invalid")?
                {
                    findings.push(
                        project_finding(relative, &bytes, native, &mut occurrences)
                            .ok_or("javadoc_finding_invalid")?,
                    );
                }
            }
        }
        sources.push(json!({"path":relative,"source_sha256":format!("{:x}",Sha256::digest(&bytes)),"reason":reason,"configuration_ref":row["configuration_ref"],"configuration_sha256":row["configuration_sha256"],"native":observation,"findings":findings}));
    }
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "clock_unavailable")?
        .as_nanos();
    Ok(
        json!({"schema_version":"0.3.0","observation_scope":if explicit {"explicit_file_probe"} else {"configured_project_probe"},"report_type":"javadoc_workbench_observation","workspace_binding":"bound","workspace_id":id,"run_id":format!("javadoc-{}-{nanos}",std::process::id()),"checker_id":"java.jdk.javadoc","authority":"local_unverified","coverage_proven":false,"delivery_decision":"not_evaluated","sources":sources}),
    )
}

/// 从固定原生规则及当前源码锚点构造问题身份；行号仅用于定位。
pub(crate) fn project_finding(
    relative: &str,
    bytes: &[u8],
    native: &Value,
    occurrences: &mut BTreeMap<String, u64>,
) -> Option<Value> {
    let rule = native["rule_id"].as_str()?;
    if !matches!(
        rule,
        "JavadocMissingComment"
            | "JavadocDefaultConstructorMissingComment"
            | "JavadocMissingParam"
            | "JavadocMissingReturn"
            | "JavadocMissingThrows"
            | "JavadocEmptyComment"
            | "JavadocMissingMainDescription"
            | "JavadocEmptyParamDescription"
            | "JavadocEmptyReturnDescription"
            | "JavadocEmptyThrowsDescription"
    ) {
        return None;
    }
    let line = native["line"].as_u64().filter(|line| *line > 0)?;
    let anchor = bytes
        .split(|b| *b == b'\n')
        .nth(usize::try_from(line - 1).ok()?)?
        .trim_ascii();
    if anchor.is_empty() {
        return None;
    }
    let mut hash = Sha256::new();
    for part in [
        b"codeguard-javadoc-finding-v1".as_slice(),
        relative.as_bytes(),
        rule.as_bytes(),
        anchor,
    ] {
        hash.update((part.len() as u64).to_be_bytes());
        hash.update(part);
    }
    let base = format!("{:x}", hash.finalize());
    let ordinal = occurrences.entry(base.clone()).or_default();
    let mut hash = Sha256::new();
    hash.update(base.as_bytes());
    hash.update(ordinal.to_be_bytes());
    *ordinal += 1;
    let fingerprint = format!("{:x}", hash.finalize());
    Some(
        json!({"finding_id":format!("CG-{}",&fingerprint[..32]),"finding_fingerprint":fingerprint,"path":relative,"source_sha256":format!("{:x}",Sha256::digest(bytes)),"rule_id":rule,"line":line}),
    )
}

#[cfg(test)]
mod tests {
    use super::prepare;
    use serde_json::{Value, json};
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    /// 构造带有效 workspace.json 的临时工作区；不执行任何 CLI 命令。
    fn workspace() -> std::path::PathBuf {
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-javadoc-workbench-unit-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join(".codeguard")).unwrap();
        let sha = "0".repeat(64);
        let document = json!({
            "schema_version":"0.3.0",
            "document_type":"codeguard_workspace",
            "workspace_id":format!("ws-{}", "1".repeat(32)),
            "project_root":".",
            "managed_files":[".gitignore","README.md","workspace.json","project.json","module-graph.json","architecture.md"],
            "project_sha256":sha,
            "module_graph_sha256":sha,
            "planned_agents_block_sha256":sha,
            "managed_sha256":{
                ".gitignore":sha,"README.md":sha,"project.json":sha,
                "module-graph.json":sha,"architecture.md":sha
            },
            "workflow_status":"initialization_partial",
            "quality_gate":"not_evaluated"
        });
        fs::write(
            root.join(".codeguard/workspace.json"),
            serde_json::to_vec(&document).unwrap(),
        )
        .unwrap();
        root
    }

    /// JDK 单文件项目探针形状的反馈；observation 为空时只需 path/reason。
    fn feedback(rows: &[Value]) -> Value {
        json!({"native_observation":{
            "schema_version":"0.4.0",
            "probe_mode":"jdk_single_file",
            "report_type":"java_javadoc_project_probe",
            "files":rows
        }})
    }

    fn row(path: &str) -> Value {
        json!({
            "path":path,
            "reason":"javadoc_configuration_not_confirmed",
            "configuration_ref":null,
            "configuration_sha256":null,
            "observation":null
        })
    }

    struct Temp(std::path::PathBuf);
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn duplicate_project_rows_are_rejected_instead_of_duplicating_findings() {
        let root = workspace();
        let _guard = Temp(root.clone());
        fs::write(root.join("Bad.java"), b"public class Bad {}\n").unwrap();
        let feedback = feedback(&[row("Bad.java"), row("Bad.java")]);
        let error = prepare(&root, &feedback).expect_err("重复路径行必须整体拒绝，不能导入重复源");
        assert_eq!(error, "javadoc_source_duplicate");
    }

    #[test]
    fn project_rows_beyond_the_total_byte_budget_are_rejected() {
        let root = workspace();
        let _guard = Temp(root.clone());
        let mut rows = Vec::new();
        for index in 0..9 {
            let name = format!("Big{index}.java");
            let file = fs::File::create(root.join(&name)).unwrap();
            // 稀疏文件：磁盘占用极小，但读取长度为 15 MiB。
            file.set_len(15 * 1024 * 1024).unwrap();
            drop(file);
            rows.push(row(&name));
        }
        let error =
            prepare(&root, &feedback(&rows)).expect_err("9 x 15 MiB 超过 128 MiB 总预算必须拒绝");
        assert_eq!(error, "javadoc_sources_byte_budget_exceeded");
        // 预算内的 8 个文件仍可导入。
        let within = prepare(&root, &feedback(&rows[..8]));
        assert!(within.is_ok(), "{within:?}");
    }

    /// 重载与同行多参数：同规则同行用序号区分，不同行用锚点区分，身份可复现。
    #[test]
    fn same_line_and_cross_line_findings_keep_distinct_stable_identities() {
        use super::project_finding;
        use std::collections::BTreeMap;
        let bytes = b"public class Bad {\n  public int add(long a, long b) { return 0; }\n}\n";
        let native = |line: u64| json!({"rule_id":"JavadocMissingParam","line":line});
        let identities = || {
            let mut occurrences = BTreeMap::new();
            (
                project_finding("Bad.java", bytes, &native(2), &mut occurrences)
                    .unwrap()
                    .clone(),
                project_finding("Bad.java", bytes, &native(2), &mut occurrences)
                    .unwrap()
                    .clone(),
            )
        };
        let (first, second) = identities();
        assert_ne!(
            first["finding_fingerprint"], second["finding_fingerprint"],
            "同一重载行上的两个缺参数诊断必须可区分"
        );
        assert_eq!((first.clone(), second.clone()), identities());
        let mut occurrences = BTreeMap::new();
        let other_line = project_finding("Bad.java", bytes, &native(1), &mut occurrences)
            .unwrap()
            .clone();
        assert_ne!(
            other_line["finding_fingerprint"], first["finding_fingerprint"],
            "不同行的重载诊断必须锚点区分"
        );
    }
}
