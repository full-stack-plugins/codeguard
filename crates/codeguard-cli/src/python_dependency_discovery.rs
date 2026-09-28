//! Python CVE 输入的只读发现；锁存在、版本 pin 与漏洞检查执行严格分离。

use std::collections::BTreeSet;
use std::path::Path;

use codeguard_adapters::{
    CheckerConfiguration, PythonRequirementsPins, inspect_python_requirements_pins,
};
use codeguard_core::ObservationPort;
use sha2::{Digest, Sha256};

use crate::discovery::{DiscoveryReport, is_standard_python_lock_name};

/// 按 Python 项目清单根记录原生漏洞检查的准备缺口，不运行 Python 或网络请求。
pub(crate) fn inspect_python_cve_inputs<P: ObservationPort>(
    root: &Path,
    observation: &P,
    report: &mut DiscoveryReport,
) {
    let Some(python) = report.languages.get("python") else {
        return;
    };
    let manifests = python.manifests.clone();
    let mut roots: BTreeSet<String> = manifests.iter().map(|path| parent(path)).collect();
    if roots.is_empty() && !python.source_files.is_empty() {
        roots.insert(".".into());
    }
    for build_root in roots {
        let locks: Vec<String> = report
            .lockfiles
            .iter()
            .filter(|path| {
                parent(path) == build_root
                    && basename(path).is_some_and(|name| {
                        matches!(name, "uv.lock" | "poetry.lock")
                            || is_standard_python_lock_name(name)
                    })
            })
            .cloned()
            .collect();
        let requirements = manifests
            .iter()
            .find(|path| parent(path) == build_root && basename(path) == Some("requirements.txt"))
            .cloned();
        let (source, reason, next_action) = if locks.len() > 1 {
            (
                build_root.clone(),
                "multiple_python_lock_inputs_unresolved",
                "确认实际使用的 Python 锁文件和构建根，再调用原生依赖及漏洞检查",
            )
        } else if let Some(lock) = locks.first() {
            if !report.lock_sha256.contains_key(lock) {
                (
                    lock.clone(),
                    "python_lock_input_unavailable",
                    "恢复可读且稳定的 Python 锁文件后重新发现，不得把缺失的依赖图当作零漏洞",
                )
            } else if basename(lock).is_some_and(is_standard_python_lock_name) {
                (
                    lock.clone(),
                    "python_standard_lock_model_unparsed",
                    "解析 PEP 751 锁文件的环境与实际包版本；核验 pip-audit 版本、--locked 原生结果及漏洞数据库身份和时效后复查",
                )
            } else {
                (
                    lock.clone(),
                    "python_lock_model_unparsed",
                    "uv.lock/poetry.lock 不能直接交给 pip-audit --locked；确认原生工具或显式导出受支持输入，并核验依赖闭包和漏洞数据库后复查",
                )
            }
        } else if let Some(requirements) = requirements {
            let state = observation
                .read_bounded(&root.join(&requirements), 256 * 1024)
                .ok()
                .filter(|bytes| {
                    report.manifest_sha256.get(&requirements)
                        == Some(&format!("{:x}", Sha256::digest(bytes)))
                })
                .map(|bytes| inspect_python_requirements_pins(&bytes));
            match state {
                Some(PythonRequirementsPins::PinnedLines) => (
                    requirements,
                    "pinned_requirements_graph_unverified",
                    "仅观察到逐行固定版本；核对传递依赖闭包、原生漏洞工具及数据库身份和时效后复查",
                ),
                Some(PythonRequirementsPins::Empty) => (
                    requirements,
                    "requirements_empty_or_unresolved",
                    "确认项目是否确无依赖及其它依赖文件；不能由空清单推断 CVE 检查通过",
                ),
                Some(PythonRequirementsPins::Unresolved) => (
                    requirements,
                    "requirements_dynamic_or_unpinned",
                    "解析引用、范围、标记或动态依赖为实际版本图后调用原生漏洞检查",
                ),
                None => {
                    report.observation_complete = false;
                    report.blocked_paths.push(requirements.clone());
                    (
                        requirements,
                        "requirements_input_unavailable_or_changed",
                        "恢复可读且稳定的依赖输入后重新发现，再运行原生漏洞检查",
                    )
                }
            }
        } else if let Some(project) = manifests.iter().find(|path| parent(path) == build_root) {
            (
                project.clone(),
                "python_project_dependencies_not_resolved",
                "解析项目清单及实际锁定的依赖图，核验原生漏洞工具和数据库后复查",
            )
        } else {
            (
                build_root.clone(),
                "python_dependency_input_not_found",
                "定位当前 Python 项目的依赖清单或锁文件，再配置并运行原生漏洞检查",
            )
        };
        report.checker_configurations.push(CheckerConfiguration {
            build_root,
            checker_id: "python.pip_audit".into(),
            category: "cve".into(),
            configuration: "unknown".into(),
            configuration_ref: source,
            reason: reason.into(),
            next_action: next_action.into(),
        });
    }
}

fn parent(path: &str) -> String {
    Path::new(path)
        .parent()
        .and_then(|parent| parent.to_str())
        .filter(|parent| !parent.is_empty())
        .unwrap_or(".")
        .into()
}

fn basename(path: &str) -> Option<&str> {
    Path::new(path).file_name()?.to_str()
}
