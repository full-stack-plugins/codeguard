use serde_json::{Value, json};
use std::{path::Path, sync::atomic::AtomicBool, time::Instant};

/// 观察选中文件的适用edition原生解析；来源：OpenSpec Rust编辑语法契约。
/// 参数为规范项目根、相对文件、显式工具及共同预算；不执行Cargo/源码或关闭任务。
pub fn observe(
    root: &Path,
    relative: &str,
    tool: &Path,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Value {
    let mut report = json!({"schema_version":"0.1.0","report_type":"rust_project_syntax_observation","status":"incomplete","reason":"rust_edition_context_unresolved","path":relative,"source_sha256":null,"edition_context":null,"native":null,"coverage_proven":false,"delivery_decision":"not_evaluated"});
    if Instant::now() >= deadline
        || cancelled.load(std::sync::atomic::Ordering::Relaxed)
        || codeguard_runtime::sigint_cancellation_requested()
    {
        report["reason"] = json!("rust_project_syntax_not_started");
        return report;
    }
    let context = match crate::rust_project_edition::RustProjectEdition::capture(root, relative) {
        Ok(context) => context,
        Err(reason) => {
            report["reason"] = json!(reason);
            return report;
        }
    };
    let source = match crate::plain_syntax_source::read_plain_source(&root.join(relative)) {
        Ok(source) => source,
        Err(reason) => {
            report["reason"] = json!(reason);
            return report;
        }
    };
    use sha2::{Digest, Sha256};
    report["source_sha256"] = json!(format!("{:x}", Sha256::digest(&source)));
    report["edition_context"] = context.report();
    if !context.current() {
        report["reason"] = json!("rust_edition_context_changed");
        return report;
    }
    let current = || {
        context.current()
            && crate::plain_syntax_source::read_plain_source(&root.join(relative))
                .ok()
                .as_deref()
                == Some(source.as_slice())
    };
    let native = crate::rustfmt_syntax_probe::observe_with_context_guard(
        tool,
        &source,
        context.edition,
        deadline,
        cancelled,
        &current,
    );
    if !context.current()
        || crate::plain_syntax_source::read_plain_source(&root.join(relative))
            .ok()
            .as_deref()
            != Some(source.as_slice())
    {
        report["reason"] = json!("rust_project_syntax_inputs_changed");
        return report;
    }
    report["status"] = native["status"].clone();
    report["reason"] = native["reason"].clone();
    report["native"] = native;
    report
}
