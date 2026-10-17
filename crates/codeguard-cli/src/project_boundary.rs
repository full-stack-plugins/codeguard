//! 项目边界与多构建根画像
//!
//! 验收标准：monorepo、多仓、worktree 和混合语言不会互相覆盖或越界观察

use serde::{Deserialize, Serialize};

/// 构建根
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuildRoot {
    /// 路径
    pub path: String,
    /// 构建系统
    pub build_system: String,
    /// 语言
    pub language: String,
}

/// 项目边界
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectBoundary {
    /// 根路径
    pub root: String,
    /// 构建根列表
    pub build_roots: Vec<BuildRoot>,
    /// 是否 monorepo
    pub is_monorepo: bool,
}

/// 边界检查器
pub struct BoundaryChecker;

impl BoundaryChecker {
    /// 创建项目边界
    pub fn create(root: &str, build_roots: Vec<BuildRoot>) -> ProjectBoundary {
        let is_monorepo = build_roots.len() > 1;
        ProjectBoundary {
            root: root.to_string(),
            build_roots,
            is_monorepo,
        }
    }
    
    /// 检查是否越界
    pub fn check_out_of_bounds(boundary: &ProjectBoundary, path: &str) -> bool {
        !path.starts_with(&boundary.root)
    }
    
    /// 检查是否互相覆盖
    pub fn check_overlap(boundary: &ProjectBoundary) -> bool {
        // 检查构建根是否有重叠
        for i in 0..boundary.build_roots.len() {
            for j in (i + 1)..boundary.build_roots.len() {
                if boundary.build_roots[i].path.starts_with(&boundary.build_roots[j].path)
                    || boundary.build_roots[j].path.starts_with(&boundary.build_roots[i].path)
                {
                    return true;
                }
            }
        }
        false
    }
}
