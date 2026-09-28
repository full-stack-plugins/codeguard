//! 原生检查语料的身份、裁定和脱敏完整性校验。

use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Component, Path, PathBuf};

/// 三种裁定状态的计数，未复现样本不得进入已接受 oracle。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CorpusCounts {
    /// 已复现且工具身份固定的样本数。
    pub accepted: usize,
    /// 仅有历史捕获、尚未复现的样本数。
    pub pending_reproduction: usize,
    /// 人工裁定仍存在争议的样本数。
    pub disputed: usize,
}

/// 读取并校验语料索引；返回可用于报告的裁定状态计数。
pub fn validate_corpus(index: &Path, plugin: &Path) -> Result<CorpusCounts, String> {
    let bytes = fs::read(index).map_err(|error| format!("读取 oracle 失败：{error}"))?;
    let document: Value =
        serde_json::from_slice(&bytes).map_err(|error| format!("oracle JSON 损坏：{error}"))?;
    let parent = index.parent().ok_or("oracle 缺父目录")?;
    validate_corpus_document(&document, parent, plugin)
}

/// 校验已解析 oracle 与当前真实文件，供正反例复用。
pub fn validate_corpus_document(
    document: &Value,
    corpus: &Path,
    plugin: &Path,
) -> Result<CorpusCounts, String> {
    if document["schema_version"] != "1.0" {
        return Err("未知语料协议版本".into());
    }
    let cases = document["cases"].as_array().ok_or("语料列表损坏")?;
    if cases.is_empty() {
        return Err("语料列表为空".into());
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut counts = CorpusCounts {
        accepted: 0,
        pending_reproduction: 0,
        disputed: 0,
    };
    for case in cases {
        let id = nonempty_string(&case["id"], "样本 ID")?;
        if !seen.insert(id) {
            return Err(format!("重复样本 ID：{id}"));
        }
        let status = nonempty_string(&case["adjudication"], "裁定状态")?;
        match status {
            "accepted" => counts.accepted += 1,
            "pending_reproduction" => counts.pending_reproduction += 1,
            "disputed" => counts.disputed += 1,
            _ => return Err(format!("{id}: 非法裁定状态")),
        }
        if status != "accepted" && case["cohort"] == "real_project_holdout" {
            return Err(format!("{id}: 未裁定样本不得进入独立 holdout"));
        }
        let relative = checked_relative(nonempty_string(&case["input"]["path"], "输入路径")?)?;
        if relative.components().count() != 1 {
            return Err(format!("{id}: 输入越出语料顶层"));
        }
        let input = corpus.join(relative);
        let actual_digest = digest_path(&input)?;
        if actual_digest != nonempty_string(&case["input"]["sha256"], "输入摘要")? {
            return Err(format!("{id}: 输入内容摘要不匹配"));
        }
        let source_ref = nonempty_string(&case["provenance"]["source_ref"], "来源引用")?;
        let source_digest = nonempty_string(&case["provenance"]["source_sha256"], "来源摘要")?;
        if source_ref.starts_with("codeguard-plugin@") {
            let source = checked_relative(nonempty_string(
                &case["provenance"]["source_file"],
                "来源文件",
            )?)?;
            if digest_path(&plugin.join(source))? != source_digest {
                return Err(format!("{id}: 插件来源摘要不匹配"));
            }
        } else if actual_digest != source_digest {
            return Err(format!("{id}: 本地样本来源摘要不匹配"));
        }
        if case["privacy"]["redacted"] != true {
            return Err(format!("{id}: 未通过脱敏复核"));
        }
        if status == "accepted"
            && (case["tool"]["version"].as_str().is_none_or(str::is_empty)
                || case["tool"]["binary_sha256"]
                    .as_str()
                    .is_none_or(str::is_empty))
        {
            return Err(format!("{id}: 已接受样本缺工具身份"));
        }
    }
    Ok(counts)
}

/// 计算普通文件或目录树的 SHA-256；拒绝符号链接以避免越界读取。
pub fn digest_path(path: &Path) -> Result<String, String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("读取 {} 失败：{error}", path.display()))?;
    if metadata.file_type().is_symlink() {
        return Err(format!("语料路径是符号链接：{}", path.display()));
    }
    let mut hasher = Sha256::new();
    if metadata.is_file() {
        hasher.update(fs::read(path).map_err(|error| error.to_string())?);
    } else if metadata.is_dir() {
        let mut members = Vec::new();
        collect_files(path, path, &mut members)?;
        members.sort();
        for relative in members {
            let display = relative.to_string_lossy().replace('\\', "/");
            hasher.update(display.as_bytes());
            hasher.update([0]);
            hasher.update(
                fs::read(path.join(&relative)).map_err(|error| format!("读取语料失败：{error}"))?,
            );
            hasher.update([0]);
        }
    } else {
        return Err(format!("非法语料文件类型：{}", path.display()));
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn collect_files(base: &Path, directory: &Path, members: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(directory).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).map_err(|error| error.to_string())?;
        if metadata.file_type().is_symlink() {
            return Err(format!("语料树含符号链接：{}", path.display()));
        }
        if metadata.is_dir() {
            collect_files(base, &path, members)?;
        } else if metadata.is_file() {
            members.push(
                path.strip_prefix(base)
                    .map_err(|error| error.to_string())?
                    .into(),
            );
        } else {
            return Err(format!("语料树含非法文件类型：{}", path.display()));
        }
    }
    Ok(())
}

fn checked_relative(value: &str) -> Result<PathBuf, String> {
    let path = Path::new(value);
    if path
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(format!("非法相对路径：{value}"));
    }
    Ok(path.to_path_buf())
}

fn nonempty_string<'a>(value: &'a Value, field: &str) -> Result<&'a str, String> {
    value
        .as_str()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("缺 {field}"))
}
