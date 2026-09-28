//! 拟入库路径的纯安全规则；不读取文件内容，也不把文件名命中称为已发现密钥。

use std::collections::BTreeSet;

/// 一条拟入库路径命中的版本库路径政策。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepositoryPathViolation {
    /// Git index 中的仓库相对路径。
    pub path: String,
    /// 稳定规则标识，供后续任务与报告关联。
    pub rule_id: String,
}

const EXCLUDED_DIRS: &[&str] = &[
    ".venv",
    "venv",
    "env",
    "node_modules",
    "vendor",
    "upstream",
    "build",
    "dist",
    "target",
    "out",
    ".next",
    ".nuxt",
    ".gradle",
    "coverage",
    ".terraform",
    ".tox",
    ".eggs",
    "htmlcov",
    ".turbo",
    ".parcel-cache",
    "__pycache__",
    ".pytest_cache",
    ".mypy_cache",
    ".ruff_cache",
    ".mimosa",
    ".worktrees",
    ".code-review-graph",
    ".kimi-code",
    ".zcode",
    ".codex-plugin",
    ".agents",
    ".idea",
    ".vscode",
];

/// 对 Git index 的路径集合实施既有仓库政策；坏路径使整个结果不可判定。
///
/// 路径必须是未经转换的 UTF-8 仓库相对路径。纯规则不会查看文件内容、
/// 读取 index 或批准例外；调用者须取得真实 Git 输入并保留完整性状态。
pub fn check_repository_paths(paths: &[String]) -> Result<Vec<RepositoryPathViolation>, String> {
    let mut seen = BTreeSet::new();
    let mut violations = Vec::new();
    for path in paths {
        if !valid_path(path) || !seen.insert(path.as_str()) {
            return Err("拟入库路径无效或重复，不能签发安全结论".into());
        }
        let parts: Vec<&str> = path.split('/').collect();
        let forbidden_directory =
            parts
                .iter()
                .enumerate()
                .take(parts.len() - 1)
                .any(|(index, part)| {
                    EXCLUDED_DIRS.contains(part)
                        && !part.starts_with('.')
                        && !(*part == "vendor" && index != 0)
                });
        let rule_id = if forbidden_directory {
            Some("repository_policy.generated_or_dependency_directory")
        } else {
            let name = parts[parts.len() - 1];
            if sensitive_filename(name) {
                Some("repository_policy.sensitive_filename")
            } else if parts.len() == 1 && root_dump_filename(name) {
                Some("repository_policy.root_dump")
            } else {
                None
            }
        };
        if let Some(rule_id) = rule_id {
            violations.push(RepositoryPathViolation {
                path: path.clone(),
                rule_id: rule_id.into(),
            });
        }
    }
    Ok(violations)
}

fn valid_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\\')
        && !path.chars().any(char::is_control)
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

fn sensitive_filename(name: &str) -> bool {
    name == ".env"
        || name.starts_with(".env.")
        || name.ends_with(".env")
        || [
            ".pem",
            ".key",
            ".p12",
            ".pfx",
            ".jks",
            ".keystore",
            ".pem.orig",
            ".pyc",
        ]
        .iter()
        .any(|suffix| name.ends_with(suffix))
        || matches!(
            name,
            "id_rsa" | "id_ed25519" | "id_ecdsa" | ".DS_Store" | "Thumbs.db"
        )
        || (name.ends_with(".json")
            && (name.starts_with("credentials")
                || name.starts_with("serviceAccount")
                || name.contains("service-account")))
}

fn root_dump_filename(name: &str) -> bool {
    [".sqlite", ".sqlite3", ".db", ".log"]
        .iter()
        .any(|suffix| name.ends_with(suffix))
}
