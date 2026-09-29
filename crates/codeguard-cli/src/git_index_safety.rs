//! 真实 Git index 路径的有界只读观察；结果不认证完整质量门禁。

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

use codeguard_core::{
    RepositoryPathViolation, check_repository_paths, has_unencrypted_openssh_ed25519_private_key,
};
use codeguard_runtime::{ProcessSpec, Termination, run_process};
use sha1::Sha1;
use sha2::{Digest, Sha256};

/// Git index 中一条未冲突的对象引用。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitIndexEntry {
    pub mode: String,
    pub oid: String,
    pub path: String,
}

/// 已核对 Git 对象头与 OID 的暂存字节摘要；不公开原始内容。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndexObjectEvidence {
    pub path: String,
    pub oid: String,
    pub content_sha256: String,
    pub size: usize,
    /// `regular`、`symlink_unresolved` 或 `lfs_pointer_unresolved`。
    pub kind: String,
}

/// 已读取同一 index 两次且路径列表一致的安全路径观察。
#[derive(Clone, Debug)]
pub struct IndexSafetyObservation {
    pub object_format: String,
    pub listing_sha256: String,
    pub entries: Vec<GitIndexEntry>,
    pub violations: Vec<RepositoryPathViolation>,
    /// 仅当全部条目为已验证普通 blob、无 Gitlink/LFS/symlink 时为 true。
    pub objects_verified: bool,
    pub object_evidence: Vec<IndexObjectEvidence>,
    pub unresolved_object_paths: Vec<String>,
    /// 对象读取或身份核验失败原因；路径违规仍保留。
    pub object_verification_reason: Option<String>,
}

/// 独立计算 Git blob 对象身份：`blob <字节长度>\0<内容>`。
#[must_use]
pub fn verify_git_blob_oid(content: &[u8], object_format: &str, expected_oid: &str) -> bool {
    let mut framed = format!("blob {}\0", content.len()).into_bytes();
    framed.extend_from_slice(content);
    match object_format {
        "sha1" => format!("{:x}", Sha1::digest(&framed)) == expected_oid,
        "sha256" => format!("{:x}", Sha256::digest(&framed)) == expected_oid,
        _ => false,
    }
}

/// 以 NUL 协议解析 `git ls-files --stage -z`；坏模式、OID、阶段或路径失败。
pub fn parse_index_listing(
    bytes: &[u8],
    object_format: &str,
) -> Result<Vec<GitIndexEntry>, String> {
    let oid_len = match object_format {
        "sha1" => 40,
        "sha256" => 64,
        _ => return Err("不支持的 Git 对象格式".into()),
    };
    if !bytes.is_empty() && !bytes.ends_with(&[0]) {
        return Err("Git index 列表缺 NUL 结束符".into());
    }
    let mut entries = Vec::new();
    for record in bytes
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty())
    {
        let Some(tab) = record.iter().position(|byte| *byte == b'\t') else {
            return Err("Git index 记录缺路径分隔符".into());
        };
        let header = std::str::from_utf8(&record[..tab]).map_err(|_| "Git index 头非 UTF-8")?;
        let fields: Vec<_> = header.split(' ').collect();
        if fields.len() != 3 || fields[2] != "0" {
            return Err("Git index 记录未合并或格式错误".into());
        }
        let mode = fields[0];
        if !matches!(mode, "100644" | "100755" | "120000" | "160000") {
            return Err("Git index 对象模式无效".into());
        }
        let oid = fields[1];
        if oid.len() != oid_len
            || oid.bytes().all(|byte| byte == b'0')
            || !oid
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err("Git index OID 无效".into());
        }
        let path = std::str::from_utf8(&record[tab + 1..])
            .map_err(|_| "Git index 路径非 UTF-8，不能无损报告")?;
        entries.push(GitIndexEntry {
            mode: mode.into(),
            oid: oid.into(),
            path: path.into(),
        });
        if entries.len() > 100_000 {
            return Err("Git index 路径超过读取上限".into());
        }
    }
    Ok(entries)
}

/// 从指定仓库根和可选替代 index 观察真实拟提交路径，不改动 index。
///
/// 本观察会核对可读取的普通 blob 字节，但 `git_tool` 的批准身份、特殊对象
/// 的完整语义及完整质量义务仍未核验；不能升级为项目交付 allow。
pub fn observe_index_safety(
    root: &Path,
    git_tool: &Path,
    alternate_index: Option<&Path>,
) -> Result<IndexSafetyObservation, String> {
    observe_index_safety_with_deadline(
        root,
        git_tool,
        alternate_index,
        Instant::now() + Duration::from_secs(15),
    )
}

/// 复用同一 Git index 观察，同时受宿主事件的总截止时间约束。
pub(crate) fn observe_index_safety_with_deadline(
    root: &Path,
    git_tool: &Path,
    alternate_index: Option<&Path>,
    deadline: Instant,
) -> Result<IndexSafetyObservation, String> {
    let root = root.canonicalize().map_err(|_| "仓库路径不可读取")?;
    if !root.is_dir() || !git_tool.is_absolute() || !git_tool.is_file() {
        return Err("仓库或 Git 工具路径无效".into());
    }
    let index = if let Some(index) = alternate_index {
        if !index.is_absolute() {
            return Err("替代 index 必须是绝对路径".into());
        }
        let metadata = fs::symlink_metadata(index).map_err(|_| "替代 index 不可读取")?;
        if !metadata.file_type().is_file() {
            return Err("替代 index 不是普通文件".into());
        }
        Some(index.to_path_buf())
    } else {
        None
    };
    let top = git(
        &root,
        git_tool,
        index.as_deref(),
        &["rev-parse", "--show-toplevel"],
        deadline,
        None,
    )?;
    let top = std::str::from_utf8(&top)
        .map_err(|_| "Git 仓库根非 UTF-8")?
        .trim_end_matches('\n');
    if Path::new(top).canonicalize().ok().as_deref() != Some(root.as_path()) {
        return Err("指定路径不是仓库根，不能缩小 index 安全范围".into());
    }
    let format = git(
        &root,
        git_tool,
        index.as_deref(),
        &["rev-parse", "--show-object-format=storage"],
        deadline,
        None,
    )?;
    let format = std::str::from_utf8(&format)
        .map_err(|_| "Git 对象格式非 UTF-8")?
        .trim_end_matches('\n');
    if !matches!(format, "sha1" | "sha256") {
        return Err("不支持的 Git 对象格式".into());
    }
    let args = ["ls-files", "--stage", "-z", "--cached"];
    let first = git(&root, git_tool, index.as_deref(), &args, deadline, None)?;
    let entries = parse_index_listing(&first, format)?;
    let paths: Vec<String> = entries.iter().map(|entry| entry.path.clone()).collect();
    let mut violations = check_repository_paths(&paths)?;
    let object_result = verify_index_objects(
        &root,
        git_tool,
        index.as_deref(),
        &entries,
        format,
        deadline,
    );
    let second = git(&root, git_tool, index.as_deref(), &args, deadline, None)?;
    if first != second {
        return Err("Git index 在观察期间变化".into());
    }
    let (object_evidence, unresolved_object_paths, object_verification_reason) = match object_result
    {
        Ok((evidence, unresolved, content_violations)) => {
            violations.extend(content_violations);
            (evidence, unresolved, None)
        }
        Err(reason) => (Vec::new(), paths.clone(), Some(reason)),
    };
    let objects_verified = object_verification_reason.is_none()
        && unresolved_object_paths.is_empty()
        && object_evidence.len() == entries.len();
    Ok(IndexSafetyObservation {
        object_format: format.into(),
        listing_sha256: format!("{:x}", Sha256::digest(&first)),
        entries,
        violations,
        objects_verified,
        object_evidence,
        unresolved_object_paths,
        object_verification_reason,
    })
}

fn verify_index_objects(
    root: &Path,
    tool: &Path,
    index: Option<&Path>,
    entries: &[GitIndexEntry],
    format: &str,
    deadline: Instant,
) -> Result<
    (
        Vec<IndexObjectEvidence>,
        Vec<String>,
        Vec<RepositoryPathViolation>,
    ),
    String,
> {
    let mut requested = BTreeMap::<&str, usize>::new();
    let mut unresolved = Vec::new();
    for entry in entries {
        if entry.mode == "160000" {
            unresolved.push(entry.path.clone());
        } else {
            requested.insert(&entry.oid, 0);
        }
    }
    if requested.is_empty() {
        return Ok((Vec::new(), unresolved, Vec::new()));
    }
    let oid_request = requested
        .keys()
        .map(|oid| format!("{oid}\n"))
        .collect::<String>();
    let checked = git(
        root,
        tool,
        index,
        &["cat-file", "--batch-check"],
        deadline,
        Some(oid_request.as_bytes()),
    )?;
    let mut lines: Vec<&[u8]> = checked.split(|byte| *byte == b'\n').collect();
    if checked.ends_with(b"\n") {
        lines.pop();
    }
    if lines.len() != requested.len() || !checked.ends_with(b"\n") {
        return Err("Git 对象大小响应数量或结束符错误".into());
    }
    let mut total = 0_usize;
    for ((oid, size), line) in requested.iter_mut().zip(lines) {
        let line = std::str::from_utf8(line).map_err(|_| "Git 对象大小响应非 UTF-8")?;
        let expected_prefix = format!("{oid} blob ");
        let count = line
            .strip_prefix(&expected_prefix)
            .ok_or("Git 对象类型或 OID 与请求不符")?
            .parse::<usize>()
            .map_err(|_| "Git 对象长度无效")?;
        if count > 8 * 1024 * 1024 {
            return Err("Git 单个对象超出本次验证预算".into());
        }
        total = total.checked_add(count).ok_or("Git 对象总长度溢出")?;
        if total > 128 * 1024 * 1024 {
            return Err("Git 对象总长度超出本次验证预算".into());
        }
        *size = count;
    }
    let mut verified = BTreeMap::<String, (usize, String, bool, bool)>::new();
    let mut batch = Vec::<(&str, usize)>::new();
    let mut batch_size = 0_usize;
    for (oid, size) in requested {
        if !batch.is_empty() && (batch.len() >= 64 || batch_size + size > 8 * 1024 * 1024) {
            read_blob_batch(root, tool, index, &batch, format, deadline, &mut verified)?;
            batch.clear();
            batch_size = 0;
        }
        batch.push((oid, size));
        batch_size += size;
    }
    if !batch.is_empty() {
        read_blob_batch(root, tool, index, &batch, format, deadline, &mut verified)?;
    }
    let mut evidence = Vec::new();
    let mut content_violations = Vec::new();
    for entry in entries {
        if entry.mode == "160000" {
            continue;
        }
        let (size, digest, lfs_pointer, private_key) =
            verified.get(&entry.oid).ok_or("Git 对象验证结果缺失")?;
        let kind = if entry.mode == "120000" {
            "symlink_unresolved"
        } else if *lfs_pointer {
            "lfs_pointer_unresolved"
        } else {
            "regular"
        };
        if kind != "regular" {
            unresolved.push(entry.path.clone());
        }
        // 仅对已核对 OID 的普通 blob 记录精确结构命中；不回显私钥字节。
        if kind == "regular" && *private_key {
            content_violations.push(RepositoryPathViolation {
                path: entry.path.clone(),
                rule_id: "repository_policy.unencrypted_openssh_ed25519_private_key".into(),
            });
        }
        evidence.push(IndexObjectEvidence {
            path: entry.path.clone(),
            oid: entry.oid.clone(),
            content_sha256: digest.clone(),
            size: *size,
            kind: kind.into(),
        });
    }
    unresolved.sort();
    Ok((evidence, unresolved, content_violations))
}

fn read_blob_batch(
    root: &Path,
    tool: &Path,
    index: Option<&Path>,
    batch: &[(&str, usize)],
    format: &str,
    deadline: Instant,
    verified: &mut BTreeMap<String, (usize, String, bool, bool)>,
) -> Result<(), String> {
    let input = batch
        .iter()
        .map(|(oid, _)| format!("{oid}\n"))
        .collect::<String>();
    let output = git(
        root,
        tool,
        index,
        &["cat-file", "--batch"],
        deadline,
        Some(input.as_bytes()),
    )?;
    let mut cursor = 0_usize;
    for (oid, size) in batch {
        let end = output[cursor..]
            .iter()
            .position(|byte| *byte == b'\n')
            .map(|offset| cursor + offset)
            .ok_or("Git batch 缺对象头结束符")?;
        let header =
            std::str::from_utf8(&output[cursor..end]).map_err(|_| "Git batch 对象头非 UTF-8")?;
        if header != format!("{oid} blob {size}") {
            return Err("Git batch 对象头身份或长度失配".into());
        }
        cursor = end + 1;
        let content_end = cursor.checked_add(*size).ok_or("Git batch 对象长度溢出")?;
        if output.get(content_end) != Some(&b'\n') {
            return Err("Git batch 对象内容或结束符截断".into());
        }
        let content = &output[cursor..content_end];
        if !verify_git_blob_oid(content, format, oid) {
            return Err("Git batch 对象字节摘要与 index OID 不符".into());
        }
        verified.insert(
            (*oid).into(),
            (
                *size,
                format!("{:x}", Sha256::digest(content)),
                content.starts_with(b"version https://git-lfs.github.com/spec/v1"),
                has_unencrypted_openssh_ed25519_private_key(content),
            ),
        );
        cursor = content_end + 1;
    }
    if cursor != output.len() {
        return Err("Git batch 含多余对象内容".into());
    }
    Ok(())
}

fn git(
    root: &Path,
    tool: &Path,
    index: Option<&Path>,
    args: &[&str],
    deadline: Instant,
    stdin: Option<&[u8]>,
) -> Result<Vec<u8>, String> {
    let mut env = BTreeMap::<OsString, OsString>::new();
    if let Some(index) = index {
        env.insert("GIT_INDEX_FILE".into(), index.as_os_str().to_owned());
    }
    let outcome = run_process(
        &ProcessSpec {
            executable: PathBuf::from(tool),
            args: args.iter().map(OsString::from).collect(),
            cwd: root.to_path_buf(),
            env,
            stdin: stdin.map(<[u8]>::to_vec),
            deadline,
            output_limit_bytes: 16 * 1024 * 1024,
        },
        &AtomicBool::new(false),
    );
    if outcome.termination != Termination::Exited(0) || !outcome.stderr.is_empty() {
        return Err("Git index 观察未完成".into());
    }
    Ok(outcome.stdout)
}
