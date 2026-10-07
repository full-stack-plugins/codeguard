//! 项目边界画像模块（9.15）。
//!
//! 实现项目边界与多构建根画像：monorepo、多仓、worktree 和混合语言不会互相覆盖或越界观察。

use serde_json::{Value, json};

/// 项目边界画像。
pub(crate) struct ProjectBoundary {
    pub build_roots: Vec<String>,
    pub languages: Vec<String>,
    pub is_monorepo: bool,
    pub is_worktree: bool,
}

/// 生成项目边界画像。
pub(crate) fn profile_boundary(
    root: &str,
    build_roots: Vec<String>,
    languages: Vec<String>,
) -> ProjectBoundary {
    let is_monorepo = build_roots.len() > 1;
    let is_worktree = root.contains(".git/worktrees");
    ProjectBoundary {
        build_roots,
        languages,
        is_monorepo,
        is_worktree,
    }
}

/// 生成边界画像报告。
pub(crate) fn boundary_report(boundary: &ProjectBoundary) -> Value {
    json!({
        "schema_version": "0.1.0",
        "report_type": "project_boundary",
        "build_roots": boundary.build_roots,
        "languages": boundary.languages,
        "is_monorepo": boundary.is_monorepo,
        "is_worktree": boundary.is_worktree,
    })
}

/// 检查边界是否越界。
pub(crate) fn check_boundary(boundary: &ProjectBoundary, target_path: &str) -> bool {
    boundary
        .build_roots
        .iter()
        .any(|root| target_path.starts_with(root.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_single_build_root() {
        let boundary =
            profile_boundary("/project", vec![".".to_string()], vec!["rust".to_string()]);
        assert!(!boundary.is_monorepo);
        assert!(!boundary.is_worktree);
    }

    #[test]
    fn profile_monorepo_detected() {
        let boundary = profile_boundary(
            "/project",
            vec!["./frontend".to_string(), "./backend".to_string()],
            vec!["typescript".to_string(), "go".to_string()],
        );
        assert!(boundary.is_monorepo);
    }

    #[test]
    fn check_boundary_detects_within() {
        let boundary = profile_boundary(
            "/project",
            vec!["./src".to_string()],
            vec!["rust".to_string()],
        );
        assert!(check_boundary(&boundary, "./src/main.rs"));
        assert!(!check_boundary(&boundary, "./tests/main.rs"));
    }

    #[test]
    fn boundary_report_contains_roots() {
        let boundary =
            profile_boundary("/project", vec![".".to_string()], vec!["rust".to_string()]);
        let report = boundary_report(&boundary);
        assert_eq!(report["build_roots"].as_array().unwrap().len(), 1);
    }
}
