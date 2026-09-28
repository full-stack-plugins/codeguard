//! 显式目录的逐文件原工具调度；保留范围缺口，不签发完整覆盖。
use crate::eslint_config_map::EslintConfigMap;
use crate::{eslint_lint_arguments::EslintLintArguments, eslint_lint_command};
use codeguard_runtime::read_bounded_regular_file;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    process::ExitCode,
    time::Instant,
};

pub(crate) fn run(args: &EslintLintArguments, deadline: Instant) -> ExitCode {
    let report = observe(args, deadline);
    if args.json {
        println!("{report}");
    } else {
        println!("尚未执行：{}", report["unexecuted_files"]);
        println!(
            "ESLint 目录检查：{}；完整覆盖与门禁尚未核验。",
            report["status"]
        );
        for file in report["files"].as_array().into_iter().flatten() {
            println!("文件：{}", file["path"]);
            eslint_lint_command::print_feedback(&file["feedback"]);
        }
        println!(
            "目录范围原因：{}；输入一致：{}",
            report["reason"], report["input_stable"]
        );
    }
    ExitCode::from(if report["reason"] == "request_cancelled" {
        130
    } else {
        3
    })
}
fn observe(args: &EslintLintArguments, deadline: Instant) -> Value {
    let mut report = json!({"schema_version":"0.2.0","report_type":"eslint_directory_feedback","status":"incomplete","authority":"local_unverified","coverage_proven":false,"delivery_decision":"not_evaluated","input_stable":false,"reason":"eslint_directory_scope_unavailable","scope_semantics":"explicit_config_per_file","config_map_sha256":null,"files":[],"skipped":[],"unexecuted_files":[]});
    let Ok(root) = args.source.canonicalize() else {
        return report;
    };
    let map = if let Some(path) = &args.config_map {
        match EslintConfigMap::load(&root, path, deadline) {
            Ok((map, digest)) => {
                report["config_map_sha256"] = json!(digest);
                report["scope_semantics"] = json!("explicit_config_map_per_file");
                Some(map)
            }
            Err(reason) => {
                report["reason"] = json!(reason);
                return report;
            }
        }
    } else {
        None
    };
    let (paths, skipped) = match inventory(&root, deadline) {
        Ok(value) => value,
        Err(reason) => {
            report["reason"] = json!(reason);
            return report;
        }
    };
    report["skipped"] = json!(skipped);
    report["unexecuted_files"] = json!(
        paths
            .iter()
            .map(|p| p.strip_prefix(&root).ok().and_then(Path::to_str))
            .collect::<Vec<_>>()
    );
    let configs = map.as_ref().map(|m| m.configs(&root)).unwrap_or_default();
    let before = match fingerprints(&paths, args, &configs, deadline) {
        Ok(value) => value,
        Err(reason) => {
            report["reason"] = json!(reason);
            return report;
        }
    };
    if args.config_map.as_ref().is_some_and(|path| {
        crate::eslint_config_map::digest(path)
            .ok()
            .is_none_or(|digest| report["config_map_sha256"] != digest)
    }) {
        report["reason"] = json!("eslint_directory_input_changed");
        return report;
    }
    let mut files = Vec::new();
    for path in &paths {
        if let Some(reason) = stopped(deadline) {
            report["reason"] = json!(reason);
            break;
        }
        let (request, scope, mapped) = if let Some(map) = &map {
            map.select(&root, path, args)
        } else {
            let mut request = args.clone();
            request.source = path.clone();
            (request, ".".into(), false)
        };
        // 子任务不能在枚举后悄悄换成另一份映射、源码或原配置。
        let map_stable = args.config_map.as_ref().is_none_or(|path| {
            crate::eslint_config_map::digest(path)
                .ok()
                .is_some_and(|digest| report["config_map_sha256"] == digest)
        });
        if !map_stable
            || !frozen(path, 16 * 1024 * 1024, &before)
            || request
                .config
                .as_ref()
                .is_none_or(|p| !frozen(p, 1024 * 1024, &before))
        {
            report["reason"] = json!("eslint_directory_input_changed");
            break;
        }
        let feedback = eslint_lint_command::observe_with_preparation(&request, deadline);
        let reason = feedback["reason"].as_str().map(str::to_owned);
        files.push(json!({"path":path.strip_prefix(&root).ok().and_then(Path::to_str),"configuration_scope":scope,"configuration_selection":if mapped {"explicit_project_map"} else {"explicit_default"},"feedback":feedback}));
        if matches!(
            reason.as_deref(),
            Some("request_cancelled" | "request_deadline_exceeded")
        ) {
            report["reason"] = json!(reason);
            break;
        }
    }
    if stopped(deadline).is_none() {
        let stable = files.len() == paths.len()
            && inventory(&root, deadline).is_ok_and(|(after, omitted)| {
                after == paths && json!(omitted) == report["skipped"]
            })
            && fingerprints(&paths, args, &configs, deadline).is_ok_and(|after| after == before)
            && args.config_map.as_ref().is_none_or(|path| {
                crate::eslint_config_map::digest(path)
                    .ok()
                    .is_some_and(|digest| report["config_map_sha256"] == digest)
            });
        report["input_stable"] = json!(stable);
        let workbench_failed = args.workspace.is_some()
            && files
                .iter()
                .any(|f| f["feedback"]["workbench_status"] != "synced_partial");
        report["reason"] = json!(if !stable {
            "eslint_directory_input_changed"
        } else if files.is_empty() {
            "eslint_directory_no_sources"
        } else if files
            .iter()
            .any(|f| f["feedback"]["local_coherent"] != true)
        {
            "eslint_directory_file_incomplete"
        } else if workbench_failed {
            "eslint_directory_workbench_incomplete"
        } else {
            "project_context_and_policy_unverified"
        });
        if stable
            && !workbench_failed
            && !files.is_empty()
            && files
                .iter()
                .all(|f| f["feedback"]["local_coherent"] == true)
        {
            report["status"] = json!("local_observation");
        }
    } else if let Some(reason) = stopped(deadline) {
        report["reason"] = json!(reason);
    }
    report["unexecuted_files"] = json!(
        paths
            .iter()
            .skip(files.len())
            .map(|p| p.strip_prefix(&root).ok().and_then(Path::to_str))
            .collect::<Vec<_>>()
    );
    report["files"] = json!(files);
    report
}
fn frozen(path: &Path, limit: u64, before: &BTreeMap<PathBuf, [u8; 32]>) -> bool {
    read_bounded_regular_file(path, limit)
        .ok()
        .is_some_and(|bytes| {
            let hash: [u8; 32] = Sha256::digest(bytes).into();
            before.get(path) == Some(&hash)
        })
}
fn stopped(deadline: Instant) -> Option<&'static str> {
    if codeguard_runtime::sigint_cancellation_requested() {
        Some("request_cancelled")
    } else if Instant::now() >= deadline {
        Some("request_deadline_exceeded")
    } else {
        None
    }
}
fn inventory(root: &Path, deadline: Instant) -> Result<(Vec<PathBuf>, Vec<Value>), &'static str> {
    let mut pending = vec![(root.to_path_buf(), 0)];
    let mut paths = Vec::new();
    let mut skipped = Vec::new();
    let mut count = 0;
    while let Some((dir, depth)) = pending.pop() {
        if let Some(reason) = stopped(deadline) {
            return Err(reason);
        }
        if depth > 64 {
            return Err("eslint_directory_depth_exceeded");
        }
        let entries = std::fs::read_dir(dir).map_err(|_| "eslint_directory_unreadable")?;
        for entry in entries {
            if let Some(reason) = stopped(deadline) {
                return Err(reason);
            }
            count += 1;
            if count > 50_000 {
                return Err("eslint_directory_entry_budget_exceeded");
            }
            let path = entry.map_err(|_| "eslint_directory_unreadable")?.path();
            let relative = path
                .strip_prefix(root)
                .ok()
                .and_then(Path::to_str)
                .filter(|s| !s.chars().any(char::is_control))
                .ok_or("eslint_directory_path_invalid")?;
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or("eslint_directory_path_invalid")?;
            let metadata =
                std::fs::symlink_metadata(&path).map_err(|_| "eslint_directory_unreadable")?;
            let skip = if metadata.file_type().is_symlink() {
                Some("symbolic_link_not_followed")
            } else if name.starts_with('.') {
                Some("hidden_entry_not_selected")
            } else if metadata.is_dir() && name == "node_modules" {
                Some("dependency_tree_not_selected")
            } else {
                None
            };
            if let Some(reason) = skip {
                skipped.push(json!({"path":relative,"reason":reason}));
            } else if metadata.is_dir() {
                pending.push((path, depth + 1));
            } else if metadata.is_file()
                && path.extension().is_some_and(|e| {
                    ["js", "jsx", "mjs", "cjs", "ts", "tsx", "mts", "cts"]
                        .iter()
                        .any(|s| e == *s)
                })
            {
                if paths.len() >= 10_000 {
                    return Err("eslint_directory_source_budget_exceeded");
                }
                paths.push(path);
            }
        }
    }
    paths.sort();
    skipped.sort_by(|a, b| a["path"].as_str().cmp(&b["path"].as_str()));
    Ok((paths, skipped))
}
fn fingerprints(
    paths: &[PathBuf],
    args: &EslintLintArguments,
    configs: &[PathBuf],
    deadline: Instant,
) -> Result<BTreeMap<PathBuf, [u8; 32]>, &'static str> {
    let mut hashes = BTreeMap::new();
    for (path, limit) in paths
        .iter()
        .map(|p| (p, 16 * 1024 * 1024))
        .chain([
            (
                args.node
                    .as_ref()
                    .ok_or("eslint_execution_context_missing")?,
                128 * 1024 * 1024,
            ),
            (
                args.entry
                    .as_ref()
                    .ok_or("eslint_execution_context_missing")?,
                16 * 1024 * 1024,
            ),
            (
                args.config
                    .as_ref()
                    .ok_or("eslint_execution_context_missing")?,
                1024 * 1024,
            ),
        ])
        .chain(configs.iter().map(|p| (p, 1024 * 1024)))
    {
        if let Some(reason) = stopped(deadline) {
            return Err(reason);
        }
        let bytes = read_bounded_regular_file(path, limit)
            .map_err(|_| "eslint_directory_input_unavailable")?;
        hashes.insert(path.clone(), Sha256::digest(bytes).into());
    }
    Ok(hashes)
}
