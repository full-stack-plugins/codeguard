use crate::checkstyle_probe::run_checkstyle_probe;
use crate::checkstyle_probe_request::CheckstyleProbeRequest;
use codeguard_adapters::checkstyle_comment_rule_bindings;
use codeguard_runtime::read_bounded_regular_file;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// 显式原配置的单文件 Checkstyle 对话反馈；不签发项目或交付结论。
pub fn run(args: &[String]) -> ExitCode {
    let Some(source) = args.first().filter(|v| !v.starts_with('-')) else {
        eprintln!("缺少 Java 源文件");
        return ExitCode::from(2);
    };
    let mut options = BTreeMap::new();
    let mut index = 1;
    while index < args.len() {
        let (key, value) = if let Some((k, v)) = args[index].split_once('=') {
            (k.to_owned(), v.to_owned())
        } else {
            let key = args[index].clone();
            index += 1;
            let Some(value) = args.get(index) else {
                eprintln!("参数缺少值");
                return ExitCode::from(2);
            };
            (key, value.clone())
        };
        if !matches!(
            key.as_str(),
            "--java-tool" | "--checkstyle-jar" | "--config" | "--format" | "--workspace"
        ) || value.is_empty()
            || options.insert(key, value).is_some()
        {
            eprintln!("参数无效或重复");
            return ExitCode::from(2);
        }
        index += 1;
    }
    let format = options
        .get("--format")
        .map(String::as_str)
        .unwrap_or("human");
    if !matches!(format, "human" | "json") {
        eprintln!("输出格式无效");
        return ExitCode::from(2);
    }
    let deadline = Instant::now() + Duration::from_secs(120);
    let mut report = json!({"schema_version":"0.4.0","report_type":"java_checkstyle_local_feedback","language":"java","operation":"lint","category":"comments","command_status":"incomplete","exit_code":3,"local_status":"incomplete","reason":"prerequisites_missing","path":source,"findings":[],"recheck_argv":[],"workbench":null,"authority":"local_unverified","checker_identity":"unverified","coverage_proven":false,"delivery_decision":"not_evaluated","next_actions":["verify_original_checkstyle_configuration_and_source_scope","bind_approved_tool_and_quality_policy"]});
    let mut input_bindings = Value::Null;
    if ["--java-tool", "--checkstyle-jar", "--config"]
        .iter()
        .all(|k| options.contains_key(*k))
    {
        match observe(source, &options, deadline) {
            Ok(observed) => {
                input_bindings = observed["input_bindings"].clone();
                report["schema_version"] = observed["schema_version"].clone();
                report["path"] = observed["path"].clone();
                report["local_status"] = observed["local_status"].clone();
                report["reason"] = observed["reason"].clone();
                report["findings"] = observed["findings"].clone();
                report["recheck_argv"] = observed["recheck_argv"].clone();
            }
            Err(reason) => report["reason"] = json!(reason),
        }
    }
    if report["reason"] == "checkstyle_configuration_regex_invalid" {
        report["next_actions"] = json!([
            "repair_original_checkstyle_regex_configuration",
            "rerun_same_native_checker_without_changing_source"
        ]);
    }
    if let Some(workspace) = options.get("--workspace") {
        report["workbench"] = match PathBuf::from(workspace).canonicalize() {
            Ok(root) if root.is_dir() => {
                crate::checkstyle_workbench::connect(&root, &report, &input_bindings)
            }
            _ => {
                json!({"status":"workspace_unavailable","new_findings":0,"new_blockers":0,"next":null})
            }
        };
    }
    if report["reason"] == "request_cancelled" {
        report["command_status"] = json!("cancelled");
        report["exit_code"] = json!(130);
    }
    if format == "json" {
        println!("{report}");
    } else {
        println!(
            "Java Checkstyle 局部注释检查：{}",
            report["local_status"].as_str().unwrap_or("incomplete")
        );
        for finding in report["findings"].as_array().into_iter().flatten() {
            println!(
                "{}:{} {} ({})",
                source,
                finding["line"],
                finding["rule_id"].as_str().unwrap_or("unknown"),
                finding["severity"].as_str().unwrap_or("unknown")
            );
            println!(
                "规则：{}",
                finding["rule_summary"].as_str().unwrap_or("unknown")
            );
            println!(
                "依据：{}",
                finding["rule_reference"].as_str().unwrap_or("unknown")
            );
            for step in finding["repair_steps"].as_array().into_iter().flatten() {
                println!("修复方向：{}", step.as_str().unwrap_or("unknown"));
            }
        }
        println!("修复工作台：{}", report["workbench"]);
        println!("复检参数：{}", report["recheck_argv"]);
        println!("待核实：{}", report["reason"].as_str().unwrap_or("unknown"));
    }
    ExitCode::from(if report["exit_code"] == 130 { 130 } else { 3 })
}

/// 在统一预算中调用显式原工具，返回冻结输入及局部诊断，不签发交付结论。
pub(crate) fn observe(
    source: &str,
    options: &BTreeMap<String, String>,
    deadline: Instant,
) -> Result<Value, &'static str> {
    let source = PathBuf::from(source)
        .canonicalize()
        .map_err(|_| "source_unavailable")?;
    if source.extension().and_then(|s| s.to_str()) != Some("java") {
        return Err("source_not_java_file");
    }
    let config = PathBuf::from(&options["--config"])
        .canonicalize()
        .map_err(|_| "configuration_unavailable")?;
    let java = PathBuf::from(&options["--java-tool"])
        .canonicalize()
        .map_err(|_| "java_tool_unavailable")?;
    let jar = PathBuf::from(&options["--checkstyle-jar"])
        .canonicalize()
        .map_err(|_| "checkstyle_jar_unavailable")?;
    let paths = [
        (&source, 16 * 1024 * 1024),
        (&config, 1024 * 1024),
        (&java, 128 * 1024 * 1024),
        (&jar, 128 * 1024 * 1024),
    ];
    let mut original = BTreeMap::new();
    let mut bytes = BTreeMap::new();
    for (path, limit) in paths {
        if Instant::now() >= deadline {
            return Err("request_deadline_exceeded");
        }
        let value = read_bounded_regular_file(path, limit).map_err(|_| "input_unavailable")?;
        original.insert(path.clone(), <[u8; 32]>::from(Sha256::digest(&value)));
        bytes.insert(path.clone(), value);
    }
    if original.len() != 4 {
        return Err("input_path_collision");
    }
    if bytes[&java].starts_with(b"#!") {
        return Err("script_launcher_requires_isolation");
    }
    let bindings = checkstyle_comment_rule_bindings(&bytes[&config], "10.21.4")
        .ok_or("checkstyle_configuration_context_unresolved")?;
    if std::str::from_utf8(&bytes[&source]).is_err() {
        return Err("source_encoding_unverified");
    }
    let temp = std::env::temp_dir()
        .canonicalize()
        .map_err(|_| "private_workspace_unavailable")?;
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "clock_unavailable")?
        .as_nanos();
    let root = temp.join(format!("cg-checkstyle-cli-{}-{nanos}", std::process::id()));
    fs::create_dir(&root).map_err(|_| "private_workspace_unavailable")?;
    let scratch = Scratch(root);
    fs::set_permissions(&scratch.0, fs::Permissions::from_mode(0o700))
        .map_err(|_| "private_workspace_unavailable")?;
    let copied_source = scratch
        .0
        .join(source.file_name().ok_or("source_unavailable")?);
    let copied_config = scratch.0.join("configuration.xml");
    fs::write(&copied_source, &bytes[&source]).map_err(|_| "snapshot_unavailable")?;
    fs::write(&copied_config, &bytes[&config]).map_err(|_| "snapshot_unavailable")?;
    let expected_sha256 = BTreeMap::from([
        (copied_source.clone(), original[&source]),
        (copied_config.clone(), original[&config]),
        (java.clone(), original[&java]),
        (jar.clone(), original[&jar]),
    ]);
    let req = CheckstyleProbeRequest {
        source: copied_source,
        config: copied_config,
        java: java.clone(),
        jar: jar.clone(),
        expected_sha256,
        report_dir: scratch.0.clone(),
        evidence_dir: scratch.0.clone(),
        run_id: "local".into(),
        expected_version: "10.21.4".into(),
        deadline,
    };
    let result = run_checkstyle_probe(&req, &AtomicBool::new(false));
    for (path, limit) in paths {
        let after = read_bounded_regular_file(path, limit).map_err(|_| "original_input_changed")?;
        if original.get(path) != Some(&<[u8; 32]>::from(Sha256::digest(after))) {
            return Err("original_input_changed");
        }
    }
    if codeguard_runtime::sigint_cancellation_requested() {
        return Err("request_cancelled");
    }
    if Instant::now() >= deadline {
        return Err("request_deadline_exceeded");
    }
    if !result.local_coherent {
        return Err(result.reason.unwrap_or("checkstyle_execution_incomplete"));
    }
    let parsed = result.parsed.ok_or("checkstyle_report_missing")?;
    let mut findings = Vec::new();
    for diagnostic in &parsed.diagnostics {
        // 自定义 ID 只从原配置关联，未知来源不能猜测成另一条规则。
        let binding = bindings
            .get(&diagnostic.source)
            .ok_or("checkstyle_rule_source_unbound")?;
        findings.push(
            json!({"path":source,"line":diagnostic.line,"column":diagnostic.column,
            "rule_id":diagnostic.source,"severity":diagnostic.severity,
            "checker_class":binding.checker_class,"rule_summary":binding.summary,
            "rule_reference":binding.rule_reference,"repair_steps":binding.repair_steps}),
        );
    }
    // 字面参数数组用于复检指引，不交给 shell 展开，也不引用临时快照。
    let recheck_argv = json!([
        "codeguard",
        "lint",
        "java",
        source,
        "--checker=checkstyle",
        "--java-tool",
        java,
        "--checkstyle-jar",
        jar,
        "--config",
        config,
        "--format=json"
    ]);
    let input_bindings = json!({"source":{"path":source,"sha256":format!("{:x}",Sha256::digest(&bytes[&source]))},
        "config":{"path":config,"sha256":format!("{:x}",Sha256::digest(&bytes[&config]))},
        "java":{"path":java,"sha256":format!("{:x}",Sha256::digest(&bytes[&java]))},
        "jar":{"path":jar,"sha256":format!("{:x}",Sha256::digest(&bytes[&jar]))}});
    Ok(
        json!({"schema_version":if bindings.values().any(|b| codeguard_adapters::checkstyle_detailed_rule_class(&b.checker_class)) {"0.5.0"}else{"0.4.0"},"path":source,"input_bindings":input_bindings,"local_status":"observed","reason":"project_configuration_and_quality_policy_unverified","findings":findings,"recheck_argv":recheck_argv}),
    )
}
