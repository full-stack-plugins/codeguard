//! 已有 Codeguard 工作区的本地所有权摘要；不构成质量策略授权。

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;

const MAX_WORKSPACE_BYTES: u64 = 128 * 1024;
const MANAGED_FILES: &[&str] = &[
    ".gitignore",
    "README.md",
    "workspace.json",
    "project.json",
    "module-graph.json",
    "architecture.md",
];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkspaceDocument {
    schema_version: String,
    document_type: String,
    #[serde(default)]
    workspace_id: Option<String>,
    project_root: String,
    managed_files: Vec<String>,
    project_sha256: String,
    module_graph_sha256: String,
    planned_agents_block_sha256: String,
    #[serde(default)]
    managed_sha256: Option<BTreeMap<String, String>>,
    workflow_status: String,
    quality_gate: String,
}

/// 已落盘的自有投影摘要，仅用于防误覆盖和恢复局部写入。
pub struct WorkspaceBaseline {
    bytes: Vec<u8>,
    workspace_id: Option<String>,
    project_sha256: String,
    module_graph_sha256: String,
    agents_block_sha256: String,
    managed_sha256: BTreeMap<String, String>,
}

impl WorkspaceBaseline {
    /// 已初始化工作区的稳定身份；旧版工作区尚无该字段。
    #[must_use]
    pub fn workspace_id(&self) -> Option<&str> {
        self.workspace_id.as_deref()
    }
    /// 上次工作区文档的原始字节，用于同次刷新比较。
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// 上次生成的 AGENTS 受管区块摘要。
    #[must_use]
    pub fn agents_block_sha256(&self) -> &str {
        &self.agents_block_sha256
    }

    /// 指定受管文件的旧摘要；旧协议只覆盖画像与模块图。
    #[must_use]
    pub fn managed_digest(&self, relative: &str) -> Option<&str> {
        match relative {
            "codeguard/project.json" => Some(&self.project_sha256),
            "codeguard/module-graph.json" => Some(&self.module_graph_sha256),
            _ => self
                .managed_sha256
                .get(relative.strip_prefix("codeguard/")?)
                .map(String::as_str),
        }
    }

    /// 新旧画像或模块图摘要不同时需要刷新；不意味着质量检查失败。
    #[must_use]
    pub fn profile_stale(&self, project: &str, graph: &str) -> bool {
        self.project_sha256 != project || self.module_graph_sha256 != graph
    }
}

/// 只读解析已初始化工作区；自写字段不可变成策略或交付权威。
pub fn read_workspace_baseline(root: &Path) -> Result<Option<WorkspaceBaseline>, String> {
    let path = root.join("codeguard/workspace.json");
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("workspace.json 不可读取".into()),
    };
    if !metadata.file_type().is_file() || metadata.len() > MAX_WORKSPACE_BYTES {
        return Err("workspace.json 不是有界普通文件".into());
    }
    let bytes = fs::read(&path).map_err(|_| "workspace.json 读取失败")?;
    if bytes.len() as u64 > MAX_WORKSPACE_BYTES {
        return Err("workspace.json 超出读取上限".into());
    }
    let document: WorkspaceDocument =
        serde_json::from_slice(&bytes).map_err(|_| "workspace.json 协议无效")?;
    if !matches!(
        document.schema_version.as_str(),
        "0.1.0" | "0.2.0" | "0.3.0"
    ) || document.document_type != "codeguard_workspace"
        || document.project_root != "."
        || document.workflow_status != "initialization_partial"
        || document.quality_gate != "not_evaluated"
        || document
            .managed_files
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            != MANAGED_FILES
        || !valid_sha256(&document.project_sha256)
        || !valid_sha256(&document.module_graph_sha256)
        || !valid_sha256(&document.planned_agents_block_sha256)
    {
        return Err("workspace.json 身份或状态无效".into());
    }
    let workspace_id = match (document.schema_version.as_str(), document.workspace_id) {
        ("0.1.0" | "0.2.0", None) => None,
        ("0.3.0", Some(id)) if valid_workspace_id(&id) => Some(id),
        _ => return Err("workspace.json 工作区 ID 无效".into()),
    };
    let managed_sha256 = match (document.schema_version.as_str(), document.managed_sha256) {
        ("0.1.0", None) => BTreeMap::new(),
        ("0.2.0" | "0.3.0", Some(map))
            if map.len() == MANAGED_FILES.len() - 1
                && MANAGED_FILES
                    .iter()
                    .filter(|name| **name != "workspace.json")
                    .all(|name| map.get(*name).is_some_and(|digest| valid_sha256(digest)))
                && map.get("project.json") == Some(&document.project_sha256)
                && map.get("module-graph.json") == Some(&document.module_graph_sha256) =>
        {
            map
        }
        _ => return Err("workspace.json 受管摘要集合无效".into()),
    };
    Ok(Some(WorkspaceBaseline {
        bytes,
        workspace_id,
        project_sha256: document.project_sha256,
        module_graph_sha256: document.module_graph_sha256,
        agents_block_sha256: document.planned_agents_block_sha256,
        managed_sha256,
    }))
}

fn valid_workspace_id(value: &str) -> bool {
    value.len() == 35
        && value.starts_with("ws-")
        && value[3..]
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
