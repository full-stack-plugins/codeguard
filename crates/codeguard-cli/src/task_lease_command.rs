//! 本地任务租约：跨进程互斥只协调同一工作区，不授予质量策略或交付权威。

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use codeguard_runtime::TaskFileLock;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::next_command::read_task_brief;
use crate::workspace_refresh::read_workspace_baseline;

const LEASE_SECONDS: u64 = 300;

struct Args {
    operation: String,
    task_id: String,
    root: PathBuf,
    owner: String,
    token: Option<String>,
    json: bool,
}

/// 原工具复检期间绑定的本地任务租约；owned 表示仅 CLI 可在结束时释放。
pub(crate) struct VerificationLease {
    pub(crate) owner: String,
    pub(crate) token: String,
    pub(crate) generation: u64,
    pub(crate) owned: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Lease {
    pub(crate) schema_version: String,
    pub(crate) task_id: String,
    pub(crate) workspace_id: String,
    pub(crate) owner: String,
    pub(crate) token_sha256: String,
    pub(crate) generation: u64,
    pub(crate) expires_at: u64,
    pub(crate) status: String,
}

/// 领取、续租或释放本地任务；成功仅表示协作状态已更新。
pub fn run(args: &[String]) -> ExitCode {
    let parsed = match parse_args(args) {
        Ok(parsed) => parsed,
        Err(reason) => {
            eprintln!("{reason}");
            return ExitCode::from(2);
        }
    };
    let result = execute(&parsed);
    match result {
        Ok(report) => {
            if parsed.json {
                println!("{report}");
            } else {
                println!(
                    "任务 {} {}；generation={}；到期时间={}；交付未评估。",
                    parsed.task_id, parsed.operation, report["generation"], report["expires_at"]
                );
                if let Some(token) = report["lease_token"].as_str() {
                    println!("lease_token={token}");
                }
            }
            ExitCode::SUCCESS
        }
        Err(reason) => {
            if parsed.json {
                println!(
                    "{}",
                    json!({"schema_version":"0.1.0", "report_type":"task_lease",
                        "operation":format!("task_{}", parsed.operation), "task_id":parsed.task_id,
                        "command_status":"incomplete", "exit_code":3, "reason":reason,
                        "lease_token":null, "generation":null, "expires_at":null,
                        "authority":"local_unverified", "delivery_decision":"not_evaluated"})
                );
            } else {
                eprintln!("任务租约未完成：{reason}");
            }
            ExitCode::from(3)
        }
    }
}

fn execute(args: &Args) -> Result<Value, &'static str> {
    let root = args.root.canonicalize().map_err(|_| "project_unreadable")?;
    if !root.is_dir() {
        return Err("project_unreadable");
    }
    let state = root.join(".codeguard/state");
    if !real_directory(&state) {
        return Err("workspace_state_unavailable");
    }
    read_task_brief(&root, &args.task_id)?;
    let locks = state.join("task_locks");
    let leases = state.join("leases");
    ensure_directory(&locks)?;
    ensure_directory(&leases)?;
    let _lock = TaskFileLock::acquire(&locks.join(format!("{}.lock", args.task_id)))
        .map_err(|_| "lease_lock_unavailable")?;
    read_task_brief(&root, &args.task_id)?;
    let baseline = read_workspace_baseline(&root).map_err(|_| "workspace_identity_unavailable")?;
    let workspace_id = baseline
        .as_ref()
        .and_then(|value| value.workspace_id())
        .ok_or("workspace_identity_unavailable")?;
    let lease_path = leases.join(format!("{}.json", args.task_id));
    let existing = read_lease(&lease_path)?;
    if existing
        .as_ref()
        .is_some_and(|lease| lease.task_id != args.task_id || lease.workspace_id != workspace_id)
    {
        return Err("lease_identity_conflict");
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "clock_unavailable")?
        .as_secs();
    let (lease, returned_token) = match args.operation.as_str() {
        "claim" => {
            if existing
                .as_ref()
                .is_some_and(|lease| lease.status == "active" && lease.expires_at > now)
            {
                return Err("task_already_claimed");
            }
            let generation = existing
                .as_ref()
                .map_or(1, |lease| lease.generation.checked_add(1).unwrap_or(0));
            if generation == 0 {
                return Err("lease_generation_exhausted");
            }
            if let Some(previous) = existing.as_ref() {
                crate::task_attempt_command::recover_abandoned(
                    &root,
                    &args.task_id,
                    previous,
                    now,
                )?;
            } else {
                crate::task_attempt_command::ensure_no_open_attempt(&root, &args.task_id)?;
            }
            let token = random_token()?;
            let lease = Lease {
                schema_version: "0.1.0".into(),
                task_id: args.task_id.clone(),
                workspace_id: workspace_id.into(),
                owner: args.owner.clone(),
                token_sha256: digest(token.as_bytes()),
                generation,
                expires_at: now.checked_add(LEASE_SECONDS).ok_or("clock_overflow")?,
                status: "active".into(),
            };
            (lease, Some(token))
        }
        "heartbeat" | "release" => {
            let mut lease = existing.ok_or("lease_not_found")?;
            let token = args.token.as_deref().ok_or("lease_token_missing")?;
            if lease.owner != args.owner || lease.token_sha256 != digest(token.as_bytes()) {
                return Err("lease_token_mismatch");
            }
            if args.operation == "release" && lease.status == "released" {
                return Ok(report(args, &lease, None));
            }
            if lease.status != "active" || lease.expires_at <= now {
                return Err("lease_expired_or_inactive");
            }
            if args.operation == "heartbeat" {
                lease.expires_at = now.checked_add(LEASE_SECONDS).ok_or("clock_overflow")?;
            } else {
                crate::task_attempt_command::ensure_no_open_attempt(&root, &args.task_id)?;
                lease.status = "released".into();
            }
            (lease, None)
        }
        _ => return Err("operation_invalid"),
    };
    write_lease(&lease_path, &lease, &state)?;
    Ok(report(args, &lease, returned_token.as_deref()))
}

/// 复检开始前借用调用方租约，或在无占用时领取短期自有租约。
pub(crate) fn begin_verification(
    root: &Path,
    task_id: &str,
    borrowed: Option<(&str, &str)>,
) -> Result<VerificationLease, &'static str> {
    if let Some((owner, token)) = borrowed {
        let (guard, generation) = lock_verification_inner(root, task_id, owner, token, None)?;
        drop(guard);
        return Ok(VerificationLease {
            owner: owner.into(),
            token: token.into(),
            generation,
            owned: false,
        });
    }
    let owner = format!(
        "codeguard-verify-{}-{}",
        std::process::id(),
        &random_token()?[..16]
    );
    let claim = Args {
        operation: "claim".into(),
        task_id: task_id.into(),
        root: root.into(),
        owner: owner.clone(),
        token: None,
        json: true,
    };
    let report = execute(&claim)?;
    let token = report["lease_token"]
        .as_str()
        .ok_or("lease_receipt_invalid")?
        .to_owned();
    let generation = report["generation"]
        .as_u64()
        .ok_or("lease_receipt_invalid")?;
    let lease = VerificationLease {
        owner,
        token,
        generation,
        owned: true,
    };
    if let Err(reason) = lock_verification(root, task_id, &lease) {
        let _ = finish_verification(root, task_id, &lease);
        return Err(reason);
    }
    Ok(lease)
}

/// 在复检结果写入时持有锁并重新核对租约，阻止过期接管与状态提交交错。
pub(crate) fn lock_verification(
    root: &Path,
    task_id: &str,
    lease: &VerificationLease,
) -> Result<TaskFileLock, &'static str> {
    let (guard, _) = lock_verification_inner(
        root,
        task_id,
        &lease.owner,
        &lease.token,
        Some(lease.generation),
    )?;
    Ok(guard)
}

fn lock_verification_inner(
    root: &Path,
    task_id: &str,
    owner: &str,
    token: &str,
    generation: Option<u64>,
) -> Result<(TaskFileLock, u64), &'static str> {
    let state = root.join(".codeguard/state");
    if !real_directory(&state) {
        return Err("workspace_state_unavailable");
    }
    let locks = state.join("task_locks");
    ensure_directory(&locks)?;
    let guard = TaskFileLock::acquire(&locks.join(format!("{task_id}.lock")))
        .map_err(|_| "lease_lock_unavailable")?;
    read_task_brief(root, task_id)?;
    crate::task_attempt_command::ensure_no_open_attempt(root, task_id)?;
    let lease =
        read_lease(&state.join(format!("leases/{task_id}.json")))?.ok_or("lease_not_found")?;
    let workspace_id = read_workspace_baseline(root)
        .map_err(|_| "workspace_identity_unavailable")?
        .and_then(|baseline| baseline.workspace_id().map(str::to_owned))
        .ok_or("workspace_identity_unavailable")?;
    if lease.task_id != task_id
        || lease.workspace_id != workspace_id
        || lease.owner != owner
        || lease.token_sha256 != digest(token.as_bytes())
        || generation.is_some_and(|expected| expected != lease.generation)
    {
        return Err("lease_token_mismatch");
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "clock_unavailable")?
        .as_secs();
    if lease.status != "active" || lease.expires_at <= now {
        return Err("lease_expired_or_inactive");
    }
    Ok((guard, lease.generation))
}

/// 仅释放 CLI 自己领取的租约；借用租约保持原样。
pub(crate) fn finish_verification(
    root: &Path,
    task_id: &str,
    lease: &VerificationLease,
) -> Result<(), &'static str> {
    if !lease.owned {
        return Ok(());
    }
    let args = Args {
        operation: "release".into(),
        task_id: task_id.into(),
        root: root.into(),
        owner: lease.owner.clone(),
        token: Some(lease.token.clone()),
        json: true,
    };
    execute(&args).map(|_| ())
}

fn report(args: &Args, lease: &Lease, token: Option<&str>) -> Value {
    json!({"schema_version":"0.1.0", "report_type":"task_lease",
        "operation":format!("task_{}", args.operation), "task_id":args.task_id,
        "command_status":"complete", "exit_code":0, "reason":null,
        "lease_token":token, "generation":lease.generation,
        "expires_at":lease.expires_at, "status":lease.status,
        "authority":"local_unverified", "delivery_decision":"not_evaluated"})
}

fn parse_args(args: &[String]) -> Result<Args, String> {
    let [operation, task_id, rest @ ..] = args else {
        return Err("task lease 缺少操作或任务 ID".into());
    };
    if !matches!(operation.as_str(), "claim" | "heartbeat" | "release") || !valid_task_id(task_id) {
        return Err("task lease 操作或任务 ID 无效".into());
    }
    let (root, options) = if rest.first().is_some_and(|value| !value.starts_with('-')) {
        (PathBuf::from(&rest[0]), &rest[1..])
    } else {
        (PathBuf::from("."), rest)
    };
    let mut owner = None;
    let mut token = None;
    let mut json = false;
    let mut index = 0;
    while index < options.len() {
        let option = options[index].as_str();
        let (key, value) = if let Some((key, value)) = option.split_once('=') {
            (key, value.to_owned())
        } else {
            index += 1;
            (
                option,
                options
                    .get(index)
                    .ok_or(format!("{option} 缺少值"))?
                    .clone(),
            )
        };
        match key {
            "--owner" if owner.is_none() && valid_owner(&value) => owner = Some(value),
            "--lease-token" if token.is_none() && valid_token(&value) => token = Some(value),
            "--format" if matches!(value.as_str(), "human" | "json") => json = value == "json",
            _ => return Err(format!("task lease 参数无效或重复：{key}")),
        }
        index += 1;
    }
    let owner = owner.ok_or("缺少或无效的 --owner")?;
    if (operation == "claim" && token.is_some()) || (operation != "claim" && token.is_none()) {
        return Err("lease-token 与操作不匹配".into());
    }
    Ok(Args {
        operation: operation.clone(),
        task_id: task_id.clone(),
        root,
        owner,
        token,
        json,
    })
}

pub(crate) fn ensure_directory(path: &Path) -> Result<(), &'static str> {
    if fs::create_dir(path).is_err() && !real_directory(path) {
        return Err("lease_directory_unavailable");
    }
    if real_directory(path) {
        Ok(())
    } else {
        Err("lease_directory_unavailable")
    }
}

pub(crate) fn real_directory(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_dir())
}

pub(crate) fn read_lease(path: &Path) -> Result<Option<Lease>, &'static str> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("lease_unreadable"),
    };
    if !metadata.file_type().is_file() || metadata.len() > 4096 || metadata.nlink() != 1 {
        return Err("lease_invalid");
    }
    let bytes = fs::read(path).map_err(|_| "lease_unreadable")?;
    let lease: Lease = serde_json::from_slice(&bytes).map_err(|_| "lease_invalid")?;
    if lease.schema_version != "0.1.0"
        || !valid_task_id(&lease.task_id)
        || !valid_owner(&lease.owner)
        || !valid_token(&lease.token_sha256)
        || lease.generation == 0
        || lease.expires_at == 0
        || !matches!(lease.status.as_str(), "active" | "released")
    {
        return Err("lease_invalid");
    }
    Ok(Some(lease))
}

fn write_lease(path: &Path, lease: &Lease, state: &Path) -> Result<(), &'static str> {
    if path.exists() {
        let metadata = fs::symlink_metadata(path).map_err(|_| "lease_invalid")?;
        if !metadata.file_type().is_file() || metadata.nlink() != 1 {
            return Err("lease_invalid");
        }
    }
    let bytes = serde_json::to_vec_pretty(lease).map_err(|_| "lease_encoding_failed")?;
    let suffix = random_token()?;
    let temp = state.join(format!(
        "lease-write-{}-{}.tmp",
        std::process::id(),
        &suffix[..16]
    ));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temp)
            .map_err(|_| "lease_staging_unavailable")?;
        file.write_all(&bytes).map_err(|_| "lease_write_failed")?;
        file.sync_all().map_err(|_| "lease_sync_failed")?;
        fs::rename(&temp, path).map_err(|_| "lease_replace_failed")?;
        File::open(path.parent().ok_or("lease_parent_missing")?)
            .and_then(|dir| dir.sync_all())
            .map_err(|_| "lease_directory_sync_failed")
    })();
    let _ = fs::remove_file(&temp);
    result
}

pub(crate) fn random_token() -> Result<String, &'static str> {
    let mut bytes = [0_u8; 32];
    File::open("/dev/urandom")
        .and_then(|mut file| file.read_exact(&mut bytes))
        .map_err(|_| "token_entropy_unavailable")?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

pub(crate) fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(crate) fn valid_task_id(value: &str) -> bool {
    let suffix = value
        .strip_prefix("CG-B-")
        .or_else(|| value.strip_prefix("CG-"));
    suffix.is_some_and(|suffix| {
        suffix.len() == 32
            && suffix
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    })
}

pub(crate) fn valid_owner(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 120
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

pub(crate) fn valid_token(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
