//! 已有工具的局部OWASP应用服务探针；非发布命令或生产资格入口。
#[cfg(unix)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use codeguard_cli::{
        gradle_dependency_check_probe::observe, gradle_dependency_check_request::Request,
        gradle_model_probe_request::Request as NativeRequest,
    };
    use std::{
        collections::BTreeSet,
        path::PathBuf,
        sync::atomic::AtomicBool,
        time::{Duration, Instant},
    };
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() < 6 {
        return Err(
            "usage: PROJECT_ROOT GRADLE_BUNDLE JAVA_HOME TASK_PATH PROJECT_FILE PROJECT_FILE..."
                .into(),
        );
    }
    let request = Request {
        module_cache: None,
        native: NativeRequest {
            project_root: PathBuf::from(&args[0]),
            gradle_bundle: PathBuf::from(&args[1]),
            java_home: PathBuf::from(&args[2]),
            project_files: args[4..].iter().map(PathBuf::from).collect::<BTreeSet<_>>(),
            deadline: Instant::now() + Duration::from_secs(90),
        },
        task_paths: vec![args[3].clone()],
    };
    let report = observe(&request, &AtomicBool::new(false));
    println!("{}", serde_json::to_string_pretty(&report)?);
    // 读取原生观察不能充当漏洞清洁或交付成功。
    std::process::exit(3);
}
#[cfg(not(unix))]
fn main() {
    eprintln!("Gradle原生执行探针尚未在此平台验收");
    std::process::exit(3);
}
