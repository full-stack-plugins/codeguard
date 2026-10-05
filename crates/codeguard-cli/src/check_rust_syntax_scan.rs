use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};
use sha2::{Digest, Sha256};

/// 只观察选中的Rust完整文件；最多64文件，共同预算，原生优先且故障不换工具。
pub(crate) fn observe(
    root: &Path,
    paths: &BTreeSet<String>,
    tool: Option<PathBuf>,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Value {
    let selection = crate::rustfmt_tool_selection::RustfmtToolSelection::discover(tool);
    let mut report = json!({"schema_version":"0.1.0","report_type":"rust_syntax_scan","scope":"single_frozen_rust_files","tool_selection":selection.report(),"source_file_count":paths.len(),"unobserved_count":paths.len().saturating_sub(64),"local_parse_complete":false,"files":[],"task_status":"not_connected","authority":"local_unverified","coverage_proven":false,"delivery_decision":"not_evaluated","native_lint_obligation":"cargo_clippy_pending"});
    let mut files = Vec::new();
    for path in paths.iter().take(64) {
        let source = crate::plain_syntax_source::read_plain_source(&root.join(path));
        let mut native =
            crate::rust_syntax_evidence::observe(root, path, selection.tool(), deadline, cancelled);
        let current = Instant::now() < deadline
            && !cancelled.load(Ordering::SeqCst)
            && source.as_ref().is_ok_and(|bytes| {
                crate::plain_syntax_source::read_plain_source(&root.join(path))
                    .ok()
                    .as_ref()
                    == Some(bytes)
            })
            && crate::rust_syntax_evidence::context_current(root, path, &native);
        if !current {
            native["diagnostics"] = json!([]);
            native["status"] = json!("incomplete");
            native["reason"] = json!(if cancelled.load(Ordering::SeqCst) {
                "request_cancelled"
            } else if Instant::now() >= deadline {
                "request_deadline_exceeded"
            } else {
                "syntax_confirmation_inputs_changed"
            });
        }
        files.push(json!({"path":path,"source_sha256":source.ok().map(|bytes|format!("{:x}",Sha256::digest(bytes))),"current":current,"tool_path":selection.tool().and_then(|p|p.canonicalize().ok()),"native":native,"task_id":null,"task_sync_reason":null,"recheck_argv":null}));
    }
    report["local_parse_complete"] = json!(
        report["unobserved_count"] == 0
            && files.iter().all(|f| f["current"] == true
                && matches!(
                    f["native"]["status"].as_str(),
                    Some("completed" | "diagnostics_observed")
                ))
    );
    report["files"] = json!(files);
    report
}
