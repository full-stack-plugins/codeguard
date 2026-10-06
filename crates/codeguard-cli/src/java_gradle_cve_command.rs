use crate::{
    check_budget::{
        budget_record, parse_check_timeout, resolve_project_default, select_check_timeout,
    },
    gradle_dependency_check_probe::observe,
    gradle_dependency_check_request::Request,
    gradle_model_probe_request::Request as NativeRequest,
};
use serde_json::json;
use std::{
    collections::BTreeSet,
    path::{Component, PathBuf},
    process::ExitCode,
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};

struct Arguments {
    module_cache: Option<PathBuf>,
    root: PathBuf,
    gradle: PathBuf,
    java: PathBuf,
    files: BTreeSet<PathBuf>,
    tasks: Vec<String>,
    timeout: Option<u64>,
    json: bool,
}

/// 执行显式Gradle OWASP原任务并反馈本轮报告；参数为CLI选项，未受信结果退出3、取消130。
/// 已初始化工作台保存脱敏准备任务；不自动安装工具，也不授予数据库时效或完整依赖覆盖。
pub fn run(args: &[String]) -> ExitCode {
    let options = match parse(args) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(2);
        }
    };
    let root = match options.root.canonicalize() {
        Ok(root) if root.is_dir() => root,
        _ => {
            eprintln!("Java Gradle项目根不可用");
            return ExitCode::from(2);
        }
    };
    let (timeout, source) = match select_check_timeout(options.timeout)
        .and_then(|selected| resolve_project_default(&root, selected.0, selected.1))
    {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(2);
        }
    };
    let cache_selected = options.module_cache.is_some();
    let workspace_root = root.clone();
    let request = Request {
        module_cache: options.module_cache,
        native: NativeRequest {
            project_root: root,
            project_files: options.files.clone(),
            gradle_bundle: options.gradle,
            java_home: options.java,
            deadline: Instant::now() + Duration::from_millis(timeout),
        },
        task_paths: options.tasks.clone(),
    };
    let native = observe(&request, &AtomicBool::new(false));
    let (task_sync, task_sync_reason) = if native["reason"] == "request_cancelled" {
        ("incomplete", Some("request_cancelled"))
    } else {
        match crate::gradle_cve_workbench::persist(
            &workspace_root,
            &options.files,
            &options.tasks,
            cache_selected,
            &native,
        ) {
            Ok(true) => ("synced", None),
            Ok(false) => ("not_initialized", None),
            Err(reason) => ("incomplete", Some(reason)),
        }
    };
    let next_action = if native["reason"] == "gradle_owasp_report_budget_exceeded" {
        "累计报告或反馈超出预算；保留完整任务清单，将原任务分批执行并汇总所有分批结果。不得删除检查义务、降低规则或把未读取报告当作无漏洞；超大单任务需要独立的大报告处理方案，当前交付仍未评估"
    } else if native["native_status"] == "reports_observed_unverified" {
        "按原任务报告调查活动与原生抑制的漏洞，核对真实依赖归属和漏洞库时效；使用相同工具、输入、原任务复检。空报告不能证明无漏洞，已初始化工作区可用next/task show查询准备任务；task verify与可信关闭仍未接线"
    } else {
        "读取原生reason恢复已有Gradle/JDK、原OWASP任务、原JSON配置和选定输入；缺插件或漏洞库属于环境阻塞，不修改无关源码。恢复后使用相同原任务复检"
    };
    let report = json!({"schema_version":"0.3.0","report_type":"java_gradle_cve_feedback","operation":"cve","language":"java","mode":"gradle","selected_inputs":options.files,"requested_task_paths":options.tasks,"budget":budget_record(timeout,source),"native":native,"next_action":next_action,"task_sync":task_sync,"task_sync_reason":task_sync_reason,"authority":"local_unverified","delivery_decision":"not_evaluated"});
    if let Some(reason) = task_sync_reason {
        if !options.json {
            eprintln!("工作台同步未完成：{reason}；核对工作区和输入后重跑原命令");
        }
    }
    if options.json {
        println!("{report}");
    } else {
        println!(
            "Java Gradle CVE：{}；交付未评估",
            report["native"]["reason"]
        );
        for task in report["native"]["reports"].as_array().into_iter().flatten() {
            println!(
                "原任务 {}：依赖观察 {} 项",
                task["task_path"], task["dependency_count"]
            );
            for advisory in task["advisories"].as_array().into_iter().flatten() {
                println!(
                    "  {} {}；原生抑制={}；依赖={}",
                    advisory["source"],
                    advisory["advisory_id"],
                    advisory["suppressed_by_native_tool"],
                    advisory["package_ids"]
                );
            }
        }
        println!("下一步：{next_action}");
    }
    ExitCode::from(if report["native"]["reason"] == "request_cancelled" {
        130
    } else {
        3
    })
}

fn parse(args: &[String]) -> Result<Arguments, String> {
    let mut module_cache = None;
    let mut root = None;
    let mut gradle = None;
    let mut java = None;
    let mut files = BTreeSet::new();
    let mut tasks = Vec::new();
    let mut timeout = None;
    let mut json = None;
    let mut seen = BTreeSet::new();
    let mut index = 0;
    while index < args.len() {
        let arg = args[index].as_str();
        if !arg.starts_with('-') {
            if root.is_some() {
                return Err("项目根重复".into());
            }
            root = Some(PathBuf::from(arg));
            index += 1;
            continue;
        }
        let (key, inline) = arg
            .split_once('=')
            .map_or((arg, None), |(key, value)| (key, Some(value)));
        if !matches!(
            key,
            "--gradle-module-cache"
                | "--gradle-bundle"
                | "--java-home"
                | "--gradle-project-file"
                | "--gradle-owasp-task"
                | "--timeout"
                | "--format"
        ) {
            return Err(format!("不支持的Java CVE参数：{key}"));
        }
        if !matches!(key, "--gradle-project-file" | "--gradle-owasp-task") && !seen.insert(key) {
            return Err(format!("参数重复：{key}"));
        }
        let value = if let Some(value) = inline {
            value
        } else {
            index += 1;
            args.get(index)
                .map(String::as_str)
                .ok_or_else(|| format!("缺参数：{key}"))?
        };
        if value.is_empty() || value.starts_with("--") {
            return Err(format!("参数无效：{key}"));
        }
        match key {
            "--gradle-module-cache" => module_cache = Some(PathBuf::from(value)),
            "--gradle-bundle" => gradle = Some(PathBuf::from(value)),
            "--java-home" => java = Some(PathBuf::from(value)),
            "--gradle-project-file" => {
                let path = PathBuf::from(value);
                if path
                    .components()
                    .any(|part| !matches!(part, Component::Normal(_)))
                    || !files.insert(path)
                {
                    return Err("选定输入越界或重复".into());
                }
            }
            "--gradle-owasp-task" => {
                if !value.starts_with(':')
                    || value.len() > 256
                    || value[1..].split(':').any(|part| {
                        part.is_empty()
                            || !part.bytes().all(|byte| {
                                byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-')
                            })
                    })
                    || tasks.iter().any(|task| task == value)
                {
                    return Err("原任务路径无效或重复".into());
                }
                tasks.push(value.to_owned());
            }
            "--timeout" => timeout = Some(parse_check_timeout(value)?),
            "--format" => {
                if !matches!(value, "human" | "json") {
                    return Err("输出格式无效".into());
                }
                json = Some(value == "json");
            }
            _ => return Err("参数无效".into()),
        }
        index += 1;
    }
    let gradle = gradle.ok_or("需要显式--gradle-bundle")?;
    let java = java.ok_or("需要显式--java-home")?;
    if module_cache
        .as_ref()
        .is_some_and(|path| !path.is_absolute())
        || !gradle.is_absolute()
        || !java.is_absolute()
        || files.is_empty()
        || files.len() > 128
        || tasks.is_empty()
        || tasks.len() > 128
    {
        return Err("需要绝对工具路径、有界选定输入及原任务路径".into());
    }
    Ok(Arguments {
        module_cache,
        root: root.unwrap_or_else(|| PathBuf::from(".")),
        gradle,
        java,
        files,
        tasks,
        timeout,
        json: json.unwrap_or(false),
    })
}
