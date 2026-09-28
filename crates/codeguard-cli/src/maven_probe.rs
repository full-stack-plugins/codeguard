//! Maven 原生命令的局部模型探测；validate 成功不证明 P3C、注释或其它质量义务。

use crate::tool_identity::{hash_bundle_tree, verify_locked_artifacts};
use crate::tool_lock::LockedTool;
use codeguard_runtime::{
    ProcessSpec, Termination, read_bounded_regular_file, run_process_recorded,
};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::Instant;

/// 明确授权的 Maven 模型探测请求；不由 detect/plan/doctor 隐式调用。
pub struct MavenProbeRequest {
    /// 含 pom.xml 的绝对项目目录。
    pub project_root: PathBuf,
    /// 已解析的原生 Maven 可执行普通文件。
    pub tool: PathBuf,
    /// Maven 可执行内容的锁定摘要。
    pub expected_tool_sha256: [u8; 32],
    /// 预期版本首行，例如 Apache Maven 3.9.16。
    pub expected_maven_line: String,
    /// 预期 Java 版本行前缀，例如 Java version: 26.0.1。
    pub expected_java_prefix: String,
    /// 可选 JDK 启动文件及锁定摘要；从工具锁构造时必填。
    pub runtime_identity: Option<(PathBuf, [u8; 32])>,
    /// 可选 Maven 委托发行包目录与锁定树摘要；从工具锁构造时必填。
    pub bundle_identity: Option<(PathBuf, String)>,
    /// 显式白名单环境；Rust runtime 不继承其它变量。
    pub environment: BTreeMap<OsString, OsString>,
    /// 调用者预建的私有证据目录。
    pub evidence_dir: PathBuf,
    /// 安全日志前缀。
    pub run_id: String,
    /// 版本探测与 validate 共用截止时间。
    pub deadline: Instant,
}

/// 从工具锁构造一次 Maven 探测所需的受控路径与环境。
pub struct MavenProbeContext {
    /// 项目根目录。
    pub project_root: PathBuf,
    /// 受管工具缓存根目录。
    pub managed_cache_root: PathBuf,
    /// 明确选定的 JDK java 可执行文件。
    pub runtime_path: PathBuf,
    /// 完整白名单环境。
    pub environment: BTreeMap<OsString, OsString>,
    /// 私有证据目录。
    pub evidence_dir: PathBuf,
    /// 安全日志前缀。
    pub run_id: String,
    /// 本次请求共享截止时间。
    pub deadline: Instant,
}

impl MavenProbeRequest {
    /// 从静态制品核验结果生成探测请求；可信锁来源仍由调用方另行证明。
    pub fn from_locked_artifacts(
        locked: &LockedTool,
        mut context: MavenProbeContext,
    ) -> Result<Self, String> {
        if locked.id != "maven" {
            return Err("not_maven_tool".into());
        }
        let runtime = locked.runtime.as_ref().ok_or("jdk_lock_missing")?;
        let bundle_lock = locked.bundle.as_ref().ok_or("bundle_lock_missing")?;
        if runtime.id != "jdk" {
            return Err("wrong_runtime_kind".into());
        }
        let verified = verify_locked_artifacts(
            locked,
            &context.project_root,
            &context.managed_cache_root,
            Some(&context.runtime_path),
        );
        if let Some(issue) = verified.tool.issue {
            return Err(issue.into());
        }
        if let Some(issue) = verified.runtime.and_then(|runtime| runtime.issue) {
            return Err(issue.into());
        }
        let bundle = verified.bundle.ok_or("bundle_lock_missing")?;
        if let Some(issue) = bundle.issue {
            return Err(issue.into());
        }
        let tool = verified.tool.path.ok_or("tool_path_missing")?;
        let java_home = context
            .environment
            .get(&OsString::from("JAVA_HOME"))
            .ok_or("java_home_missing")?;
        let expected_java = Path::new(java_home).join("bin/java");
        if fs::canonicalize(expected_java).ok().as_deref()
            != fs::canonicalize(&context.runtime_path).ok().as_deref()
        {
            return Err("java_home_not_locked_runtime".into());
        }
        context
            .environment
            .insert(OsString::from("MAVEN_SKIP_RC"), OsString::from("1"));
        Ok(Self {
            project_root: context.project_root,
            tool,
            expected_tool_sha256: decode_digest(&locked.binary_sha256)?,
            expected_maven_line: format!("Apache Maven {}", locked.version),
            expected_java_prefix: format!("Java version: {}", runtime.version),
            runtime_identity: Some((context.runtime_path, decode_digest(&runtime.binary_sha256)?)),
            bundle_identity: Some((
                bundle.path.ok_or("bundle_path_missing")?,
                bundle_lock.tree_sha256.clone(),
            )),
            environment: context.environment,
            evidence_dir: context.evidence_dir,
            run_id: context.run_id,
            deadline: context.deadline,
        })
    }
}

fn decode_digest(value: &str) -> Result<[u8; 32], String> {
    if value.len() != 64 {
        return Err("invalid_locked_digest".into());
    }
    let mut digest = [0u8; 32];
    for (index, slot) in digest.iter_mut().enumerate() {
        *slot = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
            .map_err(|_| "invalid_locked_digest")?;
    }
    Ok(digest)
}

/// Maven validate 这一局部动作的证据状态。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MavenProbeState {
    /// 原生版本与 validate 执行证据完整；不代表任何质量检查通过。
    NativeValidateComplete,
    /// 任一必要环节未确认。
    Incomplete,
}

/// Maven 探测结果；原始日志只保存在私有目录。
pub struct MavenProbeResult {
    /// 局部证据状态。
    pub state: MavenProbeState,
    /// 未完成原因；成功时为空。
    pub reason: Option<&'static str>,
    /// validate 的原生退出码；未启动或异常终止时为空。
    pub native_exit: Option<i32>,
}

/// 使用统一 Rust runtime 执行 Maven --version 与离线 validate，前后复核输入身份。
#[must_use]
pub fn run_maven_probe(request: &MavenProbeRequest, cancelled: &AtomicBool) -> MavenProbeResult {
    if request.run_id.is_empty()
        || !request
            .run_id
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
        || !request.project_root.is_absolute()
        || !request.tool.is_absolute()
        || request.expected_maven_line.trim().is_empty()
        || request.expected_java_prefix.trim().is_empty()
        || request.environment.get(&OsString::from("MAVEN_SKIP_RC")) != Some(&OsString::from("1"))
    {
        return incomplete("invalid_probe_request", None);
    }
    let pom = request.project_root.join("pom.xml");
    let Ok(pom_before) = read_bounded_regular_file(&pom, 4 * 1024 * 1024) else {
        return incomplete("pom_unavailable", None);
    };
    let pom_digest: [u8; 32] = Sha256::digest(&pom_before).into();
    if hash_tool(&request.tool) != Some(request.expected_tool_sha256) {
        return incomplete("tool_identity_mismatch", None);
    }
    if !runtime_matches(request) {
        return incomplete("runtime_identity_mismatch", None);
    }
    if !bundle_matches(request) {
        return incomplete("bundle_identity_mismatch", None);
    }

    let version = ProcessSpec {
        executable: request.tool.clone(),
        args: vec!["--version".into()],
        cwd: request.project_root.clone(),
        env: request.environment.clone(),
        stdin: None,
        deadline: request.deadline,
        output_limit_bytes: 8192,
    };
    let version_name = format!("{}-maven-version.log", request.run_id);
    let version_outcome =
        match run_process_recorded(&version, cancelled, &request.evidence_dir, &version_name) {
            Ok(outcome) => outcome,
            Err(failure) => return incomplete(failure.kind.reason(), None),
        };
    if version_outcome.termination != Termination::Exited(0) {
        // 保留执行失败类别，避免把预算、启动与管道问题合并成不可诊断的阻塞。
        let reason = match version_outcome.termination {
            Termination::TimedOut => "version_timed_out",
            Termination::DeadlineBeforeStart => "version_deadline_before_start",
            Termination::SpawnFailure => "version_spawn_failure",
            Termination::Cancelled => "version_cancelled",
            Termination::OutputLimit => "version_output_limit",
            Termination::Signaled => "version_signaled",
            Termination::InvalidSpec => "version_invalid_spec",
            Termination::ReadFailure => "version_read_failure",
            Termination::WriteFailure => "version_write_failure",
            Termination::CleanupFailure => "version_cleanup_failure",
            Termination::UnsupportedPlatform => "version_unsupported_platform",
            Termination::Exited(_) => "version_nonzero_exit",
        };
        return incomplete(reason, None);
    }
    let Ok(version_text) = std::str::from_utf8(&version_outcome.stdout) else {
        return incomplete("tool_version_mismatch", None);
    };
    let first_line = version_text.lines().next().unwrap_or("");
    if first_line != request.expected_maven_line
        && !first_line.starts_with(&format!("{} (", request.expected_maven_line))
    {
        return incomplete("tool_version_mismatch", None);
    }
    if !version_text
        .lines()
        .any(|line| line.starts_with(&request.expected_java_prefix))
    {
        return incomplete("tool_version_mismatch", None);
    }
    if hash_tool(&request.tool) != Some(request.expected_tool_sha256) {
        return incomplete("tool_identity_changed", None);
    }
    if !runtime_matches(request) {
        return incomplete("runtime_identity_changed", None);
    }
    if !bundle_matches(request) {
        return incomplete("bundle_identity_changed", None);
    }

    let validate = ProcessSpec {
        executable: request.tool.clone(),
        args: vec![
            "-B".into(),
            "-o".into(),
            "-f".into(),
            pom.as_os_str().to_os_string(),
            "validate".into(),
        ],
        cwd: request.project_root.clone(),
        env: request.environment.clone(),
        stdin: None,
        deadline: request.deadline,
        output_limit_bytes: 16 * 1024 * 1024,
    };
    let validate_name = format!("{}-maven-validate.log", request.run_id);
    let (outcome, evidence_error) =
        match run_process_recorded(&validate, cancelled, &request.evidence_dir, &validate_name) {
            Ok(outcome) => (outcome, None),
            Err(failure) => (failure.outcome, Some(failure.kind.reason())),
        };
    let native_exit = match outcome.termination {
        Termination::Exited(code) => Some(code),
        _ => None,
    };
    if let Some(reason) = evidence_error {
        return incomplete(reason, native_exit);
    }
    if hash_tool(&request.tool) != Some(request.expected_tool_sha256) {
        return incomplete("tool_identity_changed", native_exit);
    }
    if !runtime_matches(request) {
        return incomplete("runtime_identity_changed", native_exit);
    }
    if !bundle_matches(request) {
        return incomplete("bundle_identity_changed", native_exit);
    }
    let Ok(pom_after) = read_bounded_regular_file(&pom, 4 * 1024 * 1024) else {
        return incomplete("pom_changed", native_exit);
    };
    if <[u8; 32]>::from(Sha256::digest(&pom_after)) != pom_digest {
        return incomplete("pom_changed", native_exit);
    }
    if Instant::now() >= request.deadline {
        return incomplete("request_deadline_exceeded", native_exit);
    }
    match native_exit {
        Some(0) => MavenProbeResult {
            state: MavenProbeState::NativeValidateComplete,
            reason: None,
            native_exit,
        },
        Some(_) => incomplete("native_validate_failed", native_exit),
        None => incomplete("native_execution_incomplete", native_exit),
    }
}

fn hash_tool(path: &Path) -> Option<[u8; 32]> {
    let bytes = read_bounded_regular_file(path, 128 * 1024 * 1024).ok()?;
    Some(Sha256::digest(bytes).into())
}

fn runtime_matches(request: &MavenProbeRequest) -> bool {
    request
        .runtime_identity
        .as_ref()
        .is_none_or(|(path, expected)| hash_tool(path) == Some(*expected))
}

fn bundle_matches(request: &MavenProbeRequest) -> bool {
    request
        .bundle_identity
        .as_ref()
        .is_none_or(|(path, expected)| {
            hash_bundle_tree(path).is_ok_and(|actual| actual == *expected)
        })
}

fn incomplete(reason: &'static str, native_exit: Option<i32>) -> MavenProbeResult {
    MavenProbeResult {
        state: MavenProbeState::Incomplete,
        reason: Some(reason),
        native_exit,
    }
}
