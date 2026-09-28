//! 工具库存与制品只读核验；本地锁候选没有批准或门禁权限。
use crate::tool_identity::{ArtifactVerification, current_platform_id, verify_locked_artifacts};
use crate::tool_lock::parse_tool_lock_document;
use codeguard_runtime::read_bounded_regular_file;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::ExitCode;

/// 静态核验与安装预览参数；不接受工具执行选项。
struct Args {
    list: bool,
    install: bool,
    apply: bool,
    root: PathBuf,
    candidate: Option<PathBuf>,
    manifest: Option<PathBuf>,
    cache: Option<PathBuf>,
    runtimes: BTreeMap<String, PathBuf>,
    json: bool,
}

/// 列出本地声明库存或核对工具锁候选及制品；来源未核验时返回未完成退出码 3。
/// 参数支持 list/verify/install 预览，输出不启动工具或授予检查通过。
pub fn run(arguments: &[String]) -> ExitCode {
    let args = match parse_args(arguments) {
        Ok(args) => args,
        Err(reason) => {
            eprintln!("{reason}");
            return ExitCode::from(2);
        }
    };
    let mut report = json!({"schema_version":"0.1.0","report_type":"tool_artifact_inspection","inspection_status":"incomplete","authority":"unverified","readiness":"unknown","gate_effect":"none","lock_status":"unreadable","lock_sha256":null,"tools":[],"next_action":"核对项目目录及工具锁候选；核验不执行或安装工具。"});
    if args.list {
        report["report_type"] = "tool_inventory_observation".into();
        report["inventory_scope"] = "declared_lock_only".into();
        report["required_inventory"] = "unverified".into();
    }
    let mut lock_bytes = None;
    if let Ok(root) = std::fs::canonicalize(&args.root).and_then(|root| {
        if root.is_dir() {
            Ok(root)
        } else {
            Err(std::io::Error::other("not directory"))
        }
    }) {
        let candidate = args
            .candidate
            .unwrap_or_else(|| root.join("codeguard.lock.json"));
        let cache = args
            .cache
            .unwrap_or_else(|| root.join(".codeguard/cache/tools"));
        match read_bounded_regular_file(&candidate, 256 * 1024) {
            Ok(bytes) => {
                report["lock_sha256"] = format!("{:x}", Sha256::digest(&bytes)).into();
                match parse_tool_lock_document(&bytes) {
                    Ok(mut lock) => {
                        report["lock_status"] = "structurally_valid_untrusted".into();
                        lock.tools.sort_by(|left, right| {
                            (&left.id, &left.platform).cmp(&(&right.id, &right.platform))
                        });
                        let mut tools = Vec::new();
                        for locked in &lock.tools {
                            let current = current_platform_id() == Some(locked.platform.as_str());
                            if !current {
                                if args.list {
                                    let mut row = json!({"tool_id":locked.id,"platform":locked.platform,"origin_kind":locked.origin_kind,
                                        "artifact_status":"not_inspected","tool":null,"runtime":null,"bundle":null,
                                        "execution":"not_run","version_status":"lock_declared_only",
                                        "next_action":"仅声明其它平台工具；在对应平台核验制品与批准来源，不能据此判断本机缺失或就绪。"});
                                    inventory_metadata(&mut row, locked, false);
                                    if args.install {
                                        install_identity(&mut row, locked);
                                    }
                                    tools.push(row);
                                }
                                continue;
                            }
                            let runtime = locked
                                .runtime
                                .as_ref()
                                .and_then(|runtime| args.runtimes.get(&runtime.id))
                                .map(PathBuf::as_path);
                            let checked = verify_locked_artifacts(locked, &root, &cache, runtime);
                            let matched = checked.tool.issue.is_none()
                                && checked
                                    .runtime
                                    .as_ref()
                                    .is_none_or(|item| item.issue.is_none())
                                && checked
                                    .bundle
                                    .as_ref()
                                    .is_none_or(|item| item.issue.is_none());
                            let mut row = json!({"tool_id":locked.id,"platform":locked.platform,"origin_kind":locked.origin_kind,
                                "artifact_status":if matched {"matched_untrusted"} else {"incomplete"},
                                "tool":artifact(&checked.tool),"runtime":checked.runtime.as_ref().map(artifact),
                                "bundle":checked.bundle.as_ref().map(|item| json!({"digest_matched":item.digest_matched,"issue":item.issue})),
                                "execution":"not_run","version_status":"lock_declared_only",
                                "next_action":if matched {"核验工具锁批准来源及兼容要求，再由 doctor 进行有界诊断；制品匹配不能证明可启动。"} else {"按 tool/runtime/bundle 的具体 issue 恢复对应制品或定位配置，再运行 tools verify；不得改写无关源码或降低规则。"}});
                            if args.list {
                                inventory_metadata(&mut row, locked, true);
                            }
                            if args.install {
                                install_identity(&mut row, locked);
                            }
                            tools.push(row);
                        }
                        if tools.is_empty() {
                            report["next_action"] = "补齐当前平台的工具锁候选并核验来源；其它平台条目不能证明本机可用。".into();
                        }
                        report["tools"] = tools.into();
                        lock_bytes = Some(bytes);
                    }
                    Err(_) => {
                        report["lock_status"] = "invalid".into();
                        report["next_action"] = "修正工具锁协议、字段、摘要或重复身份，再核验；不要修改源码以消除锁错误。".into();
                    }
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                report["lock_status"] = "missing".into();
                report["next_action"] =
                    "提供工具、运行时与发行包身份的锁候选，再核验批准来源；不自动下载工具。".into();
            }
            Err(_) => {
                report["next_action"] =
                    "锁须是可读且不超过 256 KiB 的普通文件；拒绝符号链接，修正后重新核验。".into();
            }
        }
    }
    if args.install {
        report = install_preview(report, args.apply);
        let distribution = crate::distribution_install_preview::observe(
            args.manifest.as_deref(),
            lock_bytes.as_deref(),
        );
        attach_distribution(&mut report, distribution);
    }
    if args.json {
        println!("{report}");
    } else {
        println!(
            "{}：未完成；来源未核验；准备 unknown；门禁效力 none。",
            if args.install {
                "工具安装候选计划；未写入"
            } else if args.list {
                "声明锁工具库存；必需集合未核验"
            } else {
                "工具制品核验"
            }
        );
        println!(
            "工具锁：{}；下一步：{}",
            report["lock_status"], report["next_action"]
        );
        if args.install {
            println!(
                "发行清单：{}；原因 {}；声明绑定不等于批准。",
                report["distribution_manifest"]["status"],
                report["distribution_manifest"]["reason"]
            );
        }
        for tool in report["tools"].as_array().into_iter().flatten() {
            println!(
                "工具 {}：{}；入口 {}；运行时 {}；工具包 {}；执行 not_run",
                tool["tool_id"],
                tool["artifact_status"],
                tool["tool"],
                tool["runtime"],
                tool["bundle"]
            );
            if args.list {
                println!(
                    "声明版本 {}；平台适用 {}；声明运行时 {}；必需项未核验",
                    tool["declared_version"], tool["applicability"], tool["declared_runtime"]
                );
            }
            if args.install {
                println!(
                    "安装动作：{}；未写入；原因 {}",
                    tool["install_action"], report["reason"]
                );
                println!(
                    "发行声明：{}；只是未批准绑定，缺声明仍须准备。",
                    tool["distribution"]
                );
                if let Some(layout) = tool["distribution"].get("layout") {
                    println!(
                        "布局阶段：{}；包内容核验 not_run；下一步 {}",
                        layout["status"], layout["next_action"]
                    );
                }
            }
            println!("下一步：{}", tool["next_action"]);
        }
    }
    ExitCode::from(3)
}

fn artifact(value: &ArtifactVerification) -> Value {
    json!({"digest_matched":value.digest_matched,"executable_bit":value.executable_bit,"issue":value.issue})
}

fn parse_args(arguments: &[String]) -> Result<Args, &'static str> {
    let install = arguments.first().map(String::as_str) == Some("install");
    let list = match arguments.first().map(String::as_str) {
        Some("list" | "install") => true,
        Some("verify") => false,
        _ => return Err("tools 当前支持 list、verify 或 install 预览。"),
    };
    let mut mode = None;
    let mut root = None;
    let mut candidate = None;
    let mut cache = None;
    let mut manifest = None;
    let mut runtimes = BTreeMap::new();
    let mut format = None;
    let mut index = 1;
    while index < arguments.len() {
        let argument = &arguments[index];
        if matches!(argument.as_str(), "--apply" | "--dry-run") {
            if !install || mode.replace(argument.clone()).is_some() {
                return Err("安装模式仅用于 install 且不可重复或冲突。");
            }
        } else if argument == "--distribution-manifest" {
            if !install {
                return Err("发行清单仅用于 install。");
            }
            index += 1;
            let value = arguments.get(index).ok_or("发行清单缺少值。")?;
            if value.starts_with('-') || manifest.replace(PathBuf::from(value)).is_some() {
                return Err("发行清单须有唯一文件路径。");
            }
        } else if argument == "--lock" {
            if !install {
                return Err("--lock 仅用于 install。");
            }
            index += 1;
            let value = arguments.get(index).ok_or("锁选项缺少值。")?;
            if candidate.replace(PathBuf::from(value)).is_some() {
                return Err("锁不能重复指定。");
            }
        } else if let Some(value) = argument.strip_prefix("--format=") {
            if format.replace(value.to_owned()).is_some() {
                return Err("格式不能重复指定。");
            }
        } else if matches!(
            argument.as_str(),
            "--format" | "--tool-lock-candidate" | "--managed-cache" | "--runtime"
        ) {
            index += 1;
            let value = arguments.get(index).ok_or("选项缺少值。")?;
            match argument.as_str() {
                "--format" => {
                    if format.replace(value.clone()).is_some() {
                        return Err("格式不能重复指定。");
                    }
                }
                "--tool-lock-candidate" => {
                    if candidate.replace(PathBuf::from(value)).is_some() {
                        return Err("锁不能重复指定。");
                    }
                }
                "--managed-cache" => {
                    let path = PathBuf::from(value);
                    if !path.is_absolute() || cache.replace(path).is_some() {
                        return Err("缓存须是唯一绝对路径。");
                    }
                }
                _ => {
                    let (id, path) = value.split_once('=').ok_or("运行时须为 ID=绝对路径。")?;
                    let path = PathBuf::from(path);
                    if id.is_empty()
                        || !path.is_absolute()
                        || runtimes.insert(id.into(), path).is_some()
                    {
                        return Err("运行时须有唯一 ID 和绝对路径。");
                    }
                }
            }
        } else if argument.starts_with('-') || root.replace(PathBuf::from(argument)).is_some() {
            return Err("tools 参数或路径无效。");
        }
        index += 1;
    }
    if format
        .as_deref()
        .is_some_and(|value| !matches!(value, "human" | "json"))
    {
        return Err("格式只支持 human 或 json。");
    }
    if install && candidate.is_none() {
        return Err("tools install 须显式提供 --lock PATH。");
    }
    Ok(Args {
        install,
        apply: mode.as_deref() == Some("--apply"),
        list,
        root: root.unwrap_or_else(|| PathBuf::from(".")),
        candidate,
        manifest,
        cache,
        runtimes,
        json: format.as_deref() == Some("json"),
    })
}

// 库存声明与制品观察分开，原始路径及锁引用不进入公开反馈。
fn inventory_metadata(row: &mut Value, tool: &crate::tool_lock::LockedTool, current: bool) {
    row["applicability"] = if current {
        "current_platform"
    } else {
        "other_platform"
    }
    .into();
    row["declared_version"] = tool.version.clone().into();
    row["required_by_policy"] = Value::Null;
    row["declared_runtime"] = tool
        .runtime
        .as_ref()
        .map(|runtime| json!({"id":runtime.id,"version":runtime.version}))
        .into();
    row["adapter"] = json!({"id":tool.adapter_id,"version":tool.adapter_version});
    row["rule_source"] = json!({"id":tool.rule_source_id,"kind":tool.rule_source_kind,"sha256":tool.rule_source_sha256});
}

// 来源引用不回显宿主路径；身份由原始锁摘要和精确条目摘要约束。
fn install_identity(row: &mut Value, tool: &crate::tool_lock::LockedTool) {
    row["expected_binary_sha256"] = tool.binary_sha256.clone().into();
    row["origin_ref_sha256"] = format!("{:x}", Sha256::digest(tool.origin_ref.as_bytes())).into();
}
fn install_preview(mut report: Value, apply: bool) -> Value {
    report["schema_version"] = "0.3.0".into();
    report["report_type"] = "tool_install_preview".into();
    report["mode"] = if apply { "apply_requested" } else { "dry_run" }.into();
    report["writes_performed"] = false.into();
    report["installation_status"] = if apply {
        "blocked_before_mutation"
    } else {
        "plan_incomplete"
    }
    .into();
    report["reason"] = match report["lock_status"].as_str() {
        Some("structurally_valid_untrusted") => "approved_install_source_unbound",
        Some("missing") => "tool_lock_missing",
        Some("invalid") => "tool_lock_invalid",
        _ => "tool_lock_unreadable",
    }
    .into();
    report["missing_inputs"] = json!([
        "approved_lock_source",
        "authorized_distribution_manifest",
        "required_inventory_runtime_coverage"
    ]);
    report["planned_effects"] = json!([
        "authorized_managed_cache_only",
        "no_global_path_changes",
        "no_elevation",
        "no_project_script_execution"
    ]);
    if let Some(rows) = report["tools"].as_array_mut() {
        for row in rows {
            row["install_action"] = if row["applicability"] == "other_platform" {
                "not_applicable_on_current_platform"
            } else if row["artifact_status"] == "matched_untrusted" {
                "verify_approved_toolchain_before_doctor"
            } else {
                match row["origin_kind"].as_str() {
                    Some("project_wrapper") => "restore_approved_project_wrapper",
                    Some("system") => "prepare_authorized_system_tool_plan",
                    _ => "bind_approved_distribution_manifest",
                }
            }
            .into();
        }
    }
    report["next_action"] = "核验批准工具锁与发行清单，再形成明确缓存写入计划；当前候选不允许 apply，不通过改摘要或降级检查恢复。".into();
    report
}

// 公开每个精确绑定声明；不回显地址/路径，不把子集当成全部覆盖。
fn attach_distribution(report: &mut Value, distribution: Value) {
    if let Some(rows) = report["tools"].as_array_mut() {
        for row in rows {
            row["distribution"] = distribution["artifacts"]
                .as_array()
                .and_then(|items| {
                    items.iter().find(|a| {
                        a["tool_id"] == row["tool_id"] && a["platform"] == row["platform"]
                    })
                })
                .cloned()
                .unwrap_or(Value::Null);
        }
    }
    report["distribution_manifest"] = distribution;
}
