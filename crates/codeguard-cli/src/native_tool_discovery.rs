//! 项目本地原生工具的只读观察；不运行、安装或批准候选。

use crate::discovery::DiscoveryReport;
use crate::eslint_discovery::ESLINT_CONFIG_NAMES;
use crate::native_tool_candidate::NativeToolCandidate;
use codeguard_adapters::{
    EslintConfigState, inspect_eslint_local_candidate, inspect_maven_wrapper_candidate,
};
use codeguard_core::{ObservationPort, ObservedPathKind};
use sha2::{Digest, Sha256};
use std::io;
use std::path::{Path, PathBuf};

const MAX_LOCAL_PACKAGE_BYTES: u64 = 256 * 1024;
const MAX_WRAPPER_PROPERTIES_BYTES: u64 = 64 * 1024;

pub(crate) fn inspect_native_tools<P: ObservationPort>(
    root: &Path,
    observation: &P,
    report: &mut DiscoveryReport,
) {
    let manifests: Vec<_> = report
        .manifest_sha256
        .iter()
        .filter(|(path, _)| {
            Path::new(path)
                .file_name()
                .is_some_and(|name| name == "package.json")
        })
        .map(|(path, digest)| (path.clone(), digest.clone()))
        .collect();
    for (manifest, digest) in manifests {
        let build_root = parent(&manifest);
        let project_manifest =
            match observation.read_bounded(&root.join(&manifest), MAX_LOCAL_PACKAGE_BYTES) {
                Ok(bytes) if format!("{:x}", Sha256::digest(&bytes)) == digest => bytes,
                _ => {
                    block(report, &manifest);
                    report.native_tool_candidates.push(candidate(
                        &build_root,
                        "node.eslint",
                        "project_manifest_input_unavailable_or_changed",
                        None,
                        false,
                        "重新读取项目 package.json 并核对本轮摘要后探测本地工具",
                    ));
                    continue;
                }
            };
        let declaration_observed = inspect_eslint_local_candidate(
            Some(&project_manifest),
            None,
            None,
            EslintConfigState::Unknown,
        )
        .declared_spec
        .is_some();
        let local_root = child(&build_root, "node_modules");
        let local_manifest_path = child(&build_root, "node_modules/eslint/package.json");
        let local_entry_path = child(&build_root, "node_modules/eslint/bin/eslint.js");
        if report.tool_hint_paths.contains(&local_root)
            && matches!(
                observe_kind(root, observation, &local_root),
                KindObservation::Missing
            )
        {
            block(report, &local_root);
            report.native_tool_candidates.push(candidate(
                &build_root,
                "node.eslint",
                "local_dependency_root_changed",
                None,
                declaration_observed,
                "项目本地依赖目录在观察期间消失，重新发现后再判断工具状态",
            ));
            continue;
        }
        let local_manifest = if report.tool_hint_paths.contains(&local_root) {
            observe_file(
                root,
                observation,
                &local_manifest_path,
                MAX_LOCAL_PACKAGE_BYTES,
            )
        } else {
            FileObservation::Missing
        };
        let local_bytes = match local_manifest {
            FileObservation::Bytes(ref bytes) => Some(bytes.as_slice()),
            FileObservation::Missing => None,
            FileObservation::Untrusted => {
                block(report, &local_manifest_path);
                report.native_tool_candidates.push(candidate(
                    &build_root,
                    "node.eslint",
                    "local_package_path_untrusted",
                    None,
                    declaration_observed,
                    "核对项目本地 ESLint 包的链接和真实来源；不能据此判定工具缺失",
                ));
                continue;
            }
            FileObservation::Unreadable => {
                block(report, &local_manifest_path);
                report.native_tool_candidates.push(candidate(
                    &build_root,
                    "node.eslint",
                    "local_package_unreadable",
                    None,
                    declaration_observed,
                    "恢复项目本地 ESLint 包清单的可读性，再运行原生版本探针",
                ));
                continue;
            }
            FileObservation::Changed => {
                block(report, &local_manifest_path);
                report.native_tool_candidates.push(candidate(
                    &build_root,
                    "node.eslint",
                    "local_package_changed",
                    None,
                    declaration_observed,
                    "项目本地 ESLint 包在观察期间变化，重新发现后再运行原生探针",
                ));
                continue;
            }
        };
        let entry_kind = if local_bytes.is_some() {
            match observe_kind(root, observation, &local_entry_path) {
                KindObservation::Kind(kind) => Some(kind),
                KindObservation::Missing => None,
                KindObservation::Untrusted => Some(ObservedPathKind::Symlink),
                KindObservation::Unreadable => {
                    block(report, &local_entry_path);
                    report.native_tool_candidates.push(candidate(
                        &build_root,
                        "node.eslint",
                        "local_entry_unreadable",
                        None,
                        declaration_observed,
                        "核对项目本地 ESLint 入口路径与读取权限，再执行原生探针",
                    ));
                    continue;
                }
            }
        } else {
            None
        };
        let config_blocked = report.blocked_paths.iter().any(|path| {
            parent(path) == build_root
                && Path::new(path)
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| ESLINT_CONFIG_NAMES.contains(&name))
        });
        let config = if config_blocked
            || report.checker_configurations.iter().any(|item| {
                item.checker_id == "node.eslint"
                    && item.build_root == build_root
                    && matches!(
                        item.reason.as_str(),
                        "eslint_config_unreadable" | "eslint_manifest_invalid"
                    )
            }) {
            EslintConfigState::Invalid
        } else if report.checker_configurations.iter().any(|item| {
            item.checker_id == "node.eslint"
                && item.build_root == build_root
                && report
                    .checker_config_sha256
                    .contains_key(&item.configuration_ref)
        }) {
            EslintConfigState::Observed
        } else {
            EslintConfigState::Unknown
        };
        let result = inspect_eslint_local_candidate(
            Some(&project_manifest),
            local_bytes,
            entry_kind,
            config,
        );
        report.native_tool_candidates.push(candidate(
            &build_root,
            "node.eslint",
            result.state,
            result.observed_version,
            result.declared_spec.is_some(),
            result.next_action,
        ));
    }

    let pom_roots: Vec<_> = report
        .manifest_sha256
        .keys()
        .filter(|path| {
            Path::new(path)
                .file_name()
                .is_some_and(|name| name == "pom.xml")
        })
        .map(|path| parent(path))
        .collect();
    for build_root in pom_roots {
        let script_path = child(&build_root, "mvnw");
        let config_root = child(&build_root, ".mvn");
        if !report.tool_hint_paths.contains(&script_path)
            && !report.tool_hint_paths.contains(&config_root)
        {
            continue;
        }
        let script_kind = if report.tool_hint_paths.contains(&script_path) {
            match observe_kind(root, observation, &script_path) {
                KindObservation::Kind(kind) => Some(kind),
                KindObservation::Missing => {
                    block(report, &script_path);
                    report.native_tool_candidates.push(candidate(
                        &build_root,
                        "java.maven",
                        "wrapper_script_changed",
                        None,
                        false,
                        "Maven Wrapper 脚本在观察期间消失，重新发现后再运行探针",
                    ));
                    continue;
                }
                KindObservation::Untrusted => Some(ObservedPathKind::Symlink),
                KindObservation::Unreadable => {
                    block(report, &script_path);
                    report.native_tool_candidates.push(candidate(
                        &build_root,
                        "java.maven",
                        "wrapper_script_unreadable",
                        None,
                        false,
                        "核对 Maven Wrapper 脚本的路径和权限，再运行版本探针",
                    ));
                    continue;
                }
            }
        } else {
            None
        };
        let properties_path = child(&build_root, ".mvn/wrapper/maven-wrapper.properties");
        if report.tool_hint_paths.contains(&config_root)
            && matches!(
                observe_kind(root, observation, &config_root),
                KindObservation::Missing
            )
        {
            block(report, &config_root);
            report.native_tool_candidates.push(candidate(
                &build_root,
                "java.maven",
                "wrapper_configuration_changed",
                None,
                false,
                "Maven Wrapper 配置目录在观察期间消失，重新发现后再核对发行版本",
            ));
            continue;
        }
        let properties = if report.tool_hint_paths.contains(&config_root) {
            observe_file(
                root,
                observation,
                &properties_path,
                MAX_WRAPPER_PROPERTIES_BYTES,
            )
        } else {
            FileObservation::Missing
        };
        let property_bytes = match properties {
            FileObservation::Bytes(ref bytes) => Some(bytes.as_slice()),
            FileObservation::Missing => None,
            FileObservation::Untrusted => {
                block(report, &properties_path);
                report.native_tool_candidates.push(candidate(
                    &build_root,
                    "java.maven",
                    "wrapper_configuration_untrusted",
                    None,
                    false,
                    "核对 Maven Wrapper 配置目录及文件链接；不能把未知来源当作本地工具",
                ));
                continue;
            }
            FileObservation::Unreadable => {
                block(report, &properties_path);
                report.native_tool_candidates.push(candidate(
                    &build_root,
                    "java.maven",
                    "wrapper_configuration_unreadable",
                    None,
                    false,
                    "恢复 Maven Wrapper 配置的可读性，再核对发行版本",
                ));
                continue;
            }
            FileObservation::Changed => {
                block(report, &properties_path);
                report.native_tool_candidates.push(candidate(
                    &build_root,
                    "java.maven",
                    "wrapper_configuration_changed",
                    None,
                    false,
                    "Maven Wrapper 配置在观察期间变化，重新发现后再核对发行版本",
                ));
                continue;
            }
        };
        let result = inspect_maven_wrapper_candidate(script_kind, property_bytes);
        report.native_tool_candidates.push(candidate(
            &build_root,
            "java.maven",
            result.state,
            result.observed_version,
            false,
            result.next_action,
        ));
    }
}

fn observe_file<P: ObservationPort>(
    root: &Path,
    observation: &P,
    relative: &str,
    max_bytes: u64,
) -> FileObservation {
    match observe_kind(root, observation, relative) {
        KindObservation::Kind(ObservedPathKind::File) => {
            match observation.read_bounded(&root.join(relative), max_bytes) {
                Ok(bytes) => FileObservation::Bytes(bytes),
                Err(error) if error.kind() == io::ErrorKind::NotFound => FileObservation::Changed,
                Err(_) => FileObservation::Unreadable,
            }
        }
        KindObservation::Kind(_) => FileObservation::Untrusted,
        KindObservation::Untrusted => FileObservation::Untrusted,
        KindObservation::Missing => FileObservation::Missing,
        KindObservation::Unreadable => FileObservation::Unreadable,
    }
}

fn observe_kind<P: ObservationPort>(
    root: &Path,
    observation: &P,
    relative: &str,
) -> KindObservation {
    let mut path = PathBuf::from(root);
    let parts: Vec<_> = Path::new(relative).components().collect();
    for (index, part) in parts.iter().enumerate() {
        path.push(part);
        let final_part = index + 1 == parts.len();
        match observation.classify(&path) {
            Ok(ObservedPathKind::Directory) if !final_part => {}
            Ok(_) if !final_part => return KindObservation::Untrusted,
            Ok(kind) => return KindObservation::Kind(kind),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return KindObservation::Missing;
            }
            Err(_) => return KindObservation::Unreadable,
        }
    }
    KindObservation::Missing
}

fn block(report: &mut DiscoveryReport, path: &str) {
    report.observation_complete = false;
    if !report.blocked_paths.iter().any(|existing| existing == path) {
        report.blocked_paths.push(path.into());
    }
}

fn child(parent: &str, suffix: &str) -> String {
    if parent == "." {
        suffix.into()
    } else {
        format!("{parent}/{suffix}")
    }
}

fn parent(path: &str) -> String {
    Path::new(path)
        .parent()
        .and_then(Path::to_str)
        .filter(|path| !path.is_empty())
        .unwrap_or(".")
        .into()
}

fn candidate(
    build_root: &str,
    checker_id: &str,
    state: &str,
    observed_version: Option<String>,
    declaration_observed: bool,
    next_action: &str,
) -> NativeToolCandidate {
    NativeToolCandidate {
        build_root: build_root.into(),
        checker_id: checker_id.into(),
        state: state.into(),
        observed_version,
        declaration_observed,
        next_action: next_action.into(),
    }
}

enum FileObservation {
    Bytes(Vec<u8>),
    Missing,
    Changed,
    Untrusted,
    Unreadable,
}

enum KindObservation {
    Kind(ObservedPathKind),
    Missing,
    Untrusted,
    Unreadable,
}

#[cfg(test)]
mod tests {
    use super::{FileObservation, observe_file};
    use codeguard_core::{ObservationPort, ObservedPathKind};
    use std::io;
    use std::path::{Path, PathBuf};

    struct VanishingFile;

    impl ObservationPort for VanishingFile {
        fn classify(&self, path: &Path) -> io::Result<ObservedPathKind> {
            if path.ends_with("package.json") {
                Ok(ObservedPathKind::File)
            } else {
                Ok(ObservedPathKind::Directory)
            }
        }

        fn children(&self, _: &Path) -> io::Result<Vec<PathBuf>> {
            Ok(Vec::new())
        }

        fn read_bounded(&self, _: &Path, _: u64) -> io::Result<Vec<u8>> {
            Err(io::Error::from(io::ErrorKind::NotFound))
        }
    }

    #[test]
    fn disappearance_after_file_classification_is_changed_not_absent() {
        let observed = observe_file(
            Path::new("/fixture"),
            &VanishingFile,
            "node_modules/eslint/package.json",
            1024,
        );
        assert!(matches!(observed, FileObservation::Changed));
    }
}
