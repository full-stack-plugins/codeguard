use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    path::Path,
    sync::atomic::AtomicBool,
    time::{Instant, SystemTime, UNIX_EPOCH},
};
/// 在原任务工作区及同一文件范围执行edition适用的原生解析复检；不关闭任务。
pub(crate) fn run(
    root: &Path,
    brief: &Value,
    tool: Option<&Path>,
    deadline: Instant,
) -> Result<Value, &'static str> {
    let original = crate::syntax_task_recheck::original(root, brief)?;
    if original["language"] != "rust" {
        return Err("rustfmt_tool_does_not_match_confirmation_language");
    }
    let path = original["scope"].as_str().ok_or("syntax_scope_invalid")?;
    let selection =
        crate::rustfmt_tool_selection::RustfmtToolSelection::discover(tool.map(Path::to_path_buf));
    let source = crate::plain_syntax_source::read_plain_source(&root.join(path)).ok();
    let native = crate::rust_syntax_evidence::observe(
        root,
        path,
        selection.tool(),
        deadline,
        &AtomicBool::new(false),
    );
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "clock_unavailable")?
        .as_nanos();
    let mut report = json!({"schema_version":"0.11.0","report_type":"syntax_task_recheck","operation":"task_verify","workspace_binding":"bound","workspace_id":original["workspace_id"],"run_id":format!("syntax-native-{}-{nanos}",std::process::id()),"checker_id":"syntax.native_confirmation","task_id":brief["task_id"],"authority":"local_unverified","coverage_proven":false,"delivery_decision":"not_evaluated","target":{"path":path,"language":"rust","source_sha256":source.map(|b|format!("{:x}",Sha256::digest(b)))},"original_report":crate::syntax_task_recheck::original_reference(&original,&brief["evidence_ref"]["first_report_sha256"]),"tool_path":selection.tool().and_then(|p|p.canonicalize().ok()),"native":native,"input_stable":false});
    report["input_stable"] = json!(crate::syntax_task_recheck::inputs_current(root, &report));
    Ok(report)
}
