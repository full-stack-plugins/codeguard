//! `init` 的只读预览与受控工作区创建；准备任务和策略绑定尚未完成。

use std::collections::BTreeMap;
use std::fs::{self, OpenOptions, Permissions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicU64, Ordering};

use codeguard_adapters::legacy_registry;
use codeguard_runtime::NativeObservation;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::agents_block::{apply_agents_update, plan_agents_update, render_project_context};
use crate::discovery::{DiscoveryReport, discover};
use crate::workspace_refresh::{WorkspaceBaseline, read_workspace_baseline};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);
const DIRECTORIES: &[&str] = &[
    "codeguard",
    "codeguard/findings",
    "codeguard/tasks",
    "codeguard/decisions",
    "codeguard/reports",
    "codeguard/runs",
    "codeguard/cache",
    "codeguard/worktrees",
    "codeguard/state",
];

struct Arguments {
    root: PathBuf,
    apply: bool,
    json: bool,
}

struct PlannedFile {
    relative: &'static str,
    bytes: Vec<u8>,
}

enum ExistingFile {
    Current,
    Absent,
    OwnedOld(Vec<u8>, Permissions),
}

type ChangedPaths = (Vec<&'static str>, Vec<&'static str>);
type ApplyFailure = (Vec<&'static str>, Vec<&'static str>, String);

/// 静态发现并预览或创建受管工作区；当前部分实现始终不认证项目就绪。
pub fn run(args: &[String]) -> ExitCode {
    let parsed = match parse_args(args) {
        Ok(parsed) => parsed,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(2);
        }
    };
    let root = match parsed.root.canonicalize() {
        Ok(root) if root.is_dir() => root,
        _ => {
            eprintln!("项目根目录不可读取");
            return ExitCode::from(3);
        }
    };
    let registry = match legacy_registry() {
        Ok(registry) => registry,
        Err(_) => {
            eprintln!("内置语言清单损坏");
            return ExitCode::from(4);
        }
    };
    let discovery = discover(&root, &registry, &NativeObservation);
    let baseline = read_workspace_baseline(&root);
    let workspace_id = baseline
        .as_ref()
        .ok()
        .and_then(Option::as_ref)
        .and_then(WorkspaceBaseline::workspace_id)
        .map(str::to_owned)
        .unwrap_or_else(|| derive_workspace_id(&root));
    let (files, agents_block) = plan_files(&discovery, &workspace_id);
    let profile_stale = baseline
        .as_ref()
        .ok()
        .and_then(|baseline| baseline.as_ref())
        .filter(|_| discovery.observation_complete)
        .map(|baseline| {
            let project = files
                .iter()
                .find(|item| item.relative == "codeguard/project.json")
                .expect("固定画像");
            let graph = files
                .iter()
                .find(|item| item.relative == "codeguard/module-graph.json")
                .expect("固定模块图");
            baseline.profile_stale(
                &format!("{:x}", Sha256::digest(&project.bytes)),
                &format!("{:x}", Sha256::digest(&graph.bytes)),
            )
        });
    let mut planned_files: Vec<_> = files.iter().map(|item| item.relative).collect();
    planned_files.push("AGENTS.md");
    let (status, changed_files, changed_directories, conflict_file) = if baseline.is_err() {
        (
            "conflict",
            Vec::new(),
            Vec::new(),
            Some("codeguard/workspace.json".into()),
        )
    } else if parsed.apply {
        match apply_files(
            &root,
            &files,
            &agents_block,
            baseline.as_ref().ok().and_then(Option::as_ref),
        ) {
            Ok((files, directories)) => ("partial", files, directories, None),
            Err((files, directories, path)) if files.is_empty() && directories.is_empty() => {
                ("conflict", files, directories, Some(path))
            }
            Err((files, directories, path)) => ("partial", files, directories, Some(path)),
        }
    } else {
        ("planned", Vec::new(), Vec::new(), None)
    };
    let mut unresolved = vec![
        "profile_refresh_partial",
        "approved_quality_policy_not_bound",
        "tool_readiness_not_probed",
    ];
    if !parsed.apply || conflict_file.is_some() {
        unresolved.push("agents_managed_block_not_confirmed");
    }
    if !discovery.observation_complete {
        unresolved.push("static_discovery_incomplete");
    }
    if conflict_file.is_some() {
        unresolved.push("managed_file_conflict");
    }
    let report = json!({
        "schema_version":"0.5.0",
        "report_type":"init_plan",
        "workspace_id":workspace_id,
        "operation":"init",
        "mode":if parsed.apply { "apply" } else { "dry_run" },
        "init_status":status,
        "readiness":"unknown",
        "delivery_decision":"not_evaluated",
        "observation_complete":discovery.observation_complete,
        "profile_stale":profile_stale,
        "profile_summary":crate::init_profile_feedback::render(&discovery),
        "planned_files":planned_files,
        "changed_files":changed_files,
        "changed_directories":changed_directories,
        "conflict_file":conflict_file,
        "unresolved":unresolved,
        "next_actions":["review_project_profile", "complete_init_readiness", "run_doctor_and_check_when_available"]
    });
    if parsed.json {
        println!("{report}");
    } else {
        println!("Codeguard 初始化：{status}；检查准备：unknown；交付：未评估");
        crate::init_profile_feedback::print_human(&report["profile_summary"]);
        for item in report["changed_files"].as_array().expect("固定数组") {
            println!("已创建：{item}");
        }
        if let Some(path) = report["conflict_file"].as_str() {
            println!("冲突：{}", json!(path));
        }
    }
    if parsed.apply {
        ExitCode::from(3)
    } else {
        ExitCode::SUCCESS
    }
}

fn plan_files(discovery: &DiscoveryReport, workspace_id: &str) -> (Vec<PlannedFile>, Vec<u8>) {
    let languages: Vec<Value> = discovery
        .languages
        .iter()
        .map(|(id, evidence)| {
            json!({
                "id":id,
                "source_file_count":evidence.source_files.len(),
                "source_set_sha256":format!("{:x}", Sha256::digest(evidence.source_files.iter().cloned().collect::<Vec<_>>().join("\n").as_bytes())),
                "manifests":evidence.manifests,
                "declared_version":null,
                "installed_version":null,
                "version_status":"unknown"
            })
        })
        .collect();
    let build_roots: Vec<Value> = discovery
        .build_roots
        .iter()
        .map(|(path, manifests)| json!({"path":path,"manifests":manifests,"resolution":"static_observation_only"}))
        .collect();
    let project = json!({
        "schema_version":"0.3.0",
        "document_type":"codeguard_project_profile",
        "project_root":".",
        "observation_complete":discovery.observation_complete,
        "languages":languages,
        "build_roots":build_roots,
        "manifest_sha256":discovery.manifest_sha256,
        "package_declared_versions":discovery.declared_versions,
        "language_targets":crate::language_target_profile::render(discovery),
        "lockfiles":discovery.lockfiles,
        "lock_sha256":discovery.lock_sha256,
        "checker_config_sha256":discovery.checker_config_sha256,
        "checker_configurations":discovery.checker_configurations,
        "source_set_candidates":discovery.source_set_candidates.iter().map(|(path,language,role)| json!({"path":path,"language":language,"role":role,"status":"inferred"})).collect::<Vec<_>>(),
        "blocked_paths":discovery.blocked_paths,
        "unknown_conditions":discovery.unknown_conditions,
        "architecture_status":"unknown",
        "delivery_decision":"not_evaluated"
    });
    let graph = crate::module_graph::render(discovery);
    let project_bytes = pretty_json(&project);
    let graph_bytes = pretty_json(&graph);
    let project_sha256 = format!("{:x}", Sha256::digest(&project_bytes));
    let module_graph_sha256 = format!("{:x}", Sha256::digest(&graph_bytes));
    let agents_block =
        render_project_context(&project_sha256, &module_graph_sha256, &project, &graph);
    let mut files = vec![
        planned("codeguard/.gitignore", b"/reports/\n/runs/\n/cache/\n/worktrees/\n/state/\n".to_vec()),
        planned("codeguard/README.md", "# Codeguard 工作区\n\nproject.json 与 module-graph.json 是静态观察结果，不是批准质量策略。findings/、tasks/ 和 decisions/ 保存可审查的工作记录；reports/、runs/、cache/、worktrees/、state/ 为本地数据。任务清空不代表质量通过，修复须经原检查器复检。\n".as_bytes().to_vec()),
        planned("codeguard/project.json", project_bytes),
        planned("codeguard/module-graph.json", graph_bytes),
        planned("codeguard/architecture.md", "# 项目架构观察\n\n当前架构状态：unknown。静态清单与源码目录不足以确认 MVC、DDD 或其它架构规则；不据此启用阻断。\n\n- 项目画像：[project.json](project.json)\n- 模块关系：[module-graph.json](module-graph.json)\n- 模块依赖：见图中的直接声明及 unresolved；源码引用：unresolved。\n".as_bytes().to_vec()),
    ];
    let managed_sha256: BTreeMap<_, _> = files
        .iter()
        .map(|file| {
            (
                file.relative.strip_prefix("codeguard/").expect("固定路径"),
                format!("{:x}", Sha256::digest(&file.bytes)),
            )
        })
        .collect();
    let workspace = json!({
        "schema_version":"0.3.0",
        "document_type":"codeguard_workspace",
        "workspace_id":workspace_id,
        "project_root":".",
        "managed_files":[".gitignore","README.md","workspace.json","project.json","module-graph.json","architecture.md"],
        "project_sha256":project_sha256,
        "module_graph_sha256":module_graph_sha256,
        "planned_agents_block_sha256":format!("{:x}", Sha256::digest(&agents_block)),
        "managed_sha256":managed_sha256,
        "workflow_status":"initialization_partial",
        "quality_gate":"not_evaluated"
    });
    files.push(planned("codeguard/workspace.json", pretty_json(&workspace)));
    (files, agents_block)
}

fn derive_workspace_id(root: &Path) -> String {
    let digest = format!("{:x}", Sha256::digest(root.to_string_lossy().as_bytes()));
    format!("ws-{}", &digest[..32])
}

fn planned(relative: &'static str, bytes: Vec<u8>) -> PlannedFile {
    PlannedFile { relative, bytes }
}

fn pretty_json(value: &Value) -> Vec<u8> {
    let mut bytes = serde_json::to_vec_pretty(value).expect("静态结构可序列化");
    bytes.push(b'\n');
    bytes
}

fn apply_files(
    root: &Path,
    files: &[PlannedFile],
    agents_block: &[u8],
    baseline: Option<&WorkspaceBaseline>,
) -> Result<ChangedPaths, ApplyFailure> {
    let agents_plan = plan_agents_update(
        root,
        agents_block,
        baseline.map(WorkspaceBaseline::agents_block_sha256),
    )
    .map_err(|_| (Vec::new(), Vec::new(), "AGENTS.md".into()))?;
    let workspace = root.join("codeguard");
    if let Ok(metadata) = fs::symlink_metadata(&workspace) {
        if !metadata.file_type().is_dir() {
            return Err((Vec::new(), Vec::new(), "codeguard".into()));
        }
    }
    for item in files {
        let path = root.join(item.relative);
        if existing_file_status(&path, item, baseline).is_err() {
            return Err((Vec::new(), Vec::new(), item.relative.into()));
        }
    }
    for directory in DIRECTORIES {
        match fs::symlink_metadata(root.join(directory)) {
            Ok(metadata) if metadata.file_type().is_dir() => {}
            Ok(_) => return Err((Vec::new(), Vec::new(), (*directory).into())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err((Vec::new(), Vec::new(), (*directory).into())),
        }
    }
    let mut changed_directories = Vec::new();
    for directory in DIRECTORIES {
        let path = root.join(directory);
        match fs::create_dir(&path) {
            Ok(()) => changed_directories.push(*directory),
            Err(error) => {
                if error.kind() != std::io::ErrorKind::AlreadyExists
                    || !path.is_dir()
                    || path.is_symlink()
                {
                    return Err((Vec::new(), changed_directories, (*directory).into()));
                }
            }
        }
    }
    let mut changed = Vec::new();
    for item in files
        .iter()
        .filter(|item| item.relative != "codeguard/workspace.json")
    {
        let target = root.join(item.relative);
        let status = match existing_file_status(&target, item, baseline) {
            Ok(ExistingFile::Current) => continue,
            Ok(status) => status,
            Err(_) => return Err((changed, changed_directories, item.relative.into())),
        };
        if write_planned(&workspace.join("state"), &target, &item.bytes, status).is_err() {
            return Err((changed, changed_directories, item.relative.into()));
        }
        changed.push(item.relative);
    }
    match apply_agents_update(root, &workspace.join("state"), &agents_plan) {
        Ok(true) => changed.push("AGENTS.md"),
        Ok(false) => {}
        Err(_) => return Err((changed, changed_directories, "AGENTS.md".into())),
    }
    let item = files
        .iter()
        .find(|item| item.relative == "codeguard/workspace.json")
        .expect("固定工作区文档");
    let target = root.join(item.relative);
    let status = match existing_file_status(&target, item, baseline) {
        Ok(status) => status,
        Err(_) => return Err((changed, changed_directories, item.relative.into())),
    };
    if !matches!(status, ExistingFile::Current) {
        if write_planned(&workspace.join("state"), &target, &item.bytes, status).is_err() {
            return Err((changed, changed_directories, item.relative.into()));
        }
        changed.push(item.relative);
    }
    Ok((changed, changed_directories))
}

fn existing_file_status(
    path: &Path,
    item: &PlannedFile,
    baseline: Option<&WorkspaceBaseline>,
) -> Result<ExistingFile, String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() && metadata.len() <= 8 * 1024 * 1024 => {
            let bytes = fs::read(path).map_err(|_| "受管文件不可读取")?;
            if bytes == item.bytes {
                Ok(ExistingFile::Current)
            } else if (item.relative == "codeguard/workspace.json"
                && baseline.is_some_and(|old| old.bytes() == bytes))
                || baseline
                    .and_then(|old| old.managed_digest(item.relative))
                    .is_some_and(|digest| format!("{:x}", Sha256::digest(&bytes)) == digest)
            {
                Ok(ExistingFile::OwnedOld(bytes, metadata.permissions()))
            } else {
                Err("受管文件内容冲突".into())
            }
        }
        Ok(_) => Err("受管文件类型或长度冲突".into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(ExistingFile::Absent),
        Err(_) => Err("受管文件不可读取".into()),
    }
}

fn write_planned(
    state: &Path,
    target: &Path,
    bytes: &[u8],
    status: ExistingFile,
) -> Result<(), String> {
    match status {
        ExistingFile::Current => Ok(()),
        ExistingFile::Absent => create_new_atomic(state, target, bytes),
        ExistingFile::OwnedOld(prior, permissions) => {
            replace_owned_atomic(state, target, bytes, &prior, permissions)
        }
    }
}

fn replace_owned_atomic(
    state: &Path,
    target: &Path,
    bytes: &[u8],
    prior: &[u8],
    permissions: Permissions,
) -> Result<(), String> {
    let temp_id = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
    let temp = state.join(format!(".refresh-{}-{temp_id}", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(|_| "刷新暂存文件不可创建")?;
    let result = file
        .write_all(bytes)
        .and_then(|()| file.set_permissions(permissions))
        .and_then(|()| file.sync_all());
    drop(file);
    if result.is_err() || fs::read(target).ok().as_deref() != Some(prior) {
        let _ = fs::remove_file(&temp);
        return Err("受管文件在刷新前变化或暂存失败".into());
    }
    let renamed = fs::rename(&temp, target);
    let _ = fs::remove_file(&temp);
    renamed.map_err(|_| "受管文件替换失败".to_owned())?;
    if fs::read(target).ok().as_deref() != Some(bytes) {
        return Err("受管文件替换后身份不符".into());
    }
    Ok(())
}

fn create_new_atomic(state: &Path, target: &Path, bytes: &[u8]) -> Result<(), String> {
    let temp_id = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
    let temp = state.join(format!(".init-{}-{temp_id}", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(|_| "初始化暂存文件不可创建")?;
    let result = file
        .write_all(bytes)
        .and_then(|()| file.sync_all())
        .and_then(|()| fs::hard_link(&temp, target));
    drop(file);
    let _ = fs::remove_file(&temp);
    result.map_err(|_| "受管文件创建冲突或失败".into())
}

fn parse_args(args: &[String]) -> Result<Arguments, String> {
    let mut root = None;
    let mut apply = false;
    let mut mode_seen = false;
    let mut json = false;
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if arg == "--apply" || arg == "--dry-run" {
            if mode_seen {
                return Err("初始化模式重复".into());
            }
            apply = arg == "--apply";
            mode_seen = true;
        } else if arg == "--format" {
            index += 1;
            json = parse_format(args.get(index).ok_or("--format 缺少值")?)?;
        } else if let Some(value) = arg.strip_prefix("--format=") {
            json = parse_format(value)?;
        } else if arg.starts_with('-') || root.is_some() {
            return Err(format!("不支持的参数：{arg}"));
        } else {
            root = Some(PathBuf::from(arg));
        }
        index += 1;
    }
    Ok(Arguments {
        root: root.unwrap_or_else(|| PathBuf::from(".")),
        apply,
        json,
    })
}

fn parse_format(value: &str) -> Result<bool, String> {
    match value {
        "json" => Ok(true),
        "human" => Ok(false),
        _ => Err("--format 仅支持 human/json".into()),
    }
}
