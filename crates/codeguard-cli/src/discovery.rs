//! 只读项目发现；候选不等于已验证的检查能力。

use crate::eslint_discovery::{ESLINT_CONFIG_NAMES, inspect_node_eslint};
use crate::native_tool_candidate::NativeToolCandidate;
use crate::native_tool_discovery::inspect_native_tools;
use crate::npm_audit_discovery::inspect_node_npm_audit;
use crate::python_dependency_discovery::inspect_python_cve_inputs;
use codeguard_adapters::{
    CargoModuleModel, CheckerConfiguration, LegacyRegistry, MavenModuleModel,
    inspect_gradle_unknown, inspect_maven_pom, inspect_maven_unreadable, inspect_ruff_config,
    inspect_ruff_missing, inspect_ruff_unknown, parse_package_json,
};
use codeguard_core::{ObservationPort, ObservedPathKind};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const MAX_ENTRIES: usize = 100_000;
const MAX_MANIFEST_BYTES: u64 = 256 * 1024;
const LOCK_NAMES: &[&str] = &[
    "Cargo.lock",
    "package-lock.json",
    "pnpm-lock.yaml",
    "yarn.lock",
    "poetry.lock",
    "uv.lock",
    "gradle.lockfile",
    "Gemfile.lock",
    "go.sum",
    "composer.lock",
];

/// 某语言的源码与清单证据。
#[derive(Debug, Default, Eq, PartialEq)]
pub struct LanguageEvidence {
    pub source_files: BTreeSet<String>,
    pub manifests: BTreeSet<String>,
}

/// 静态观察结果；无法读取的范围必须显式保留。
#[derive(Debug)]
pub struct DiscoveryReport {
    pub root: String,
    pub observation_complete: bool,
    pub languages: BTreeMap<String, LanguageEvidence>,
    /// 后缀不足以判定语言的源码；仍须进入候选范围的未路由计数。
    pub(crate) ambiguous_source_files: BTreeSet<String>,
    pub build_roots: BTreeMap<String, BTreeSet<String>>,
    pub declared_versions: BTreeMap<String, String>,
    /// 与同次清单字节摘要绑定的 Maven 直接模块声明。
    pub maven_module_models: BTreeMap<String, MavenModuleModel>,
    /// 与同次清单字节摘要绑定的 Cargo 直接模块声明。
    pub cargo_module_models: BTreeMap<String, CargoModuleModel>,
    pub manifest_sha256: BTreeMap<String, String>,
    pub lock_sha256: BTreeMap<String, String>,
    /// 已识别原生检查器配置文件的原始字节身份；解析状态单独记录。
    pub checker_config_sha256: BTreeMap<String, String>,
    pub lockfiles: BTreeSet<String>,
    pub checker_configurations: Vec<CheckerConfiguration>,
    /// 项目本地原生工具的只读候选；不能视为本轮工具检查成功。
    pub native_tool_candidates: Vec<NativeToolCandidate>,
    pub(crate) tool_hint_paths: BTreeSet<String>,
    ruff_config_files: BTreeSet<String>,
    eslint_config_files: BTreeSet<String>,
    /// (源码根、语言、用途)；仅按目录约定推断。
    pub source_set_candidates: BTreeSet<(String, String, String)>,
    pub blocked_paths: Vec<String>,
    pub unknown_conditions: Vec<String>,
    pub observed_entries: usize,
    /// 普通发现按既有点前缀策略跳过的路径根数量，不展开其中子项。
    pub dot_prefix_roots_excluded: usize,
    /// 按配置发现例外实际观察到的点前缀普通文件数。
    pub configuration_exception_files_observed: usize,
    /// 普通源码扫描跳过的依赖目录根数；固定路径工具探测仍可只读观察。
    pub dependency_roots_excluded: usize,
}

impl DiscoveryReport {
    /// 转成稳定的机器报告结构。
    pub fn to_json(&self) -> Value {
        let languages: Vec<Value> = self
            .languages
            .iter()
            .map(|(id, evidence)| {
                json!({"id": id, "source_files": evidence.source_files, "manifests": evidence.manifests})
            })
            .collect();
        let build_roots: Vec<Value> = self
            .build_roots
            .iter()
            .map(|(path, manifests)| json!({"path":path,"manifests":manifests}))
            .collect();
        let source_set_candidates: Vec<Value> = self
            .source_set_candidates
            .iter()
            .map(|(path, language, role)| {
                json!({"path":path,"language":language,"role":role,"basis":"directory_convention"})
            })
            .collect();
        json!({
            "schema_version":"0.4.0",
            "report_type":"discovery",
            "root":self.root,
            "observation_complete":self.observation_complete,
            "observed_entries":self.observed_entries,
            "scope_summary":{
                "ordinary_scan_policy":"project_sources_default_v2",
                "dot_prefix_roots_excluded":self.dot_prefix_roots_excluded,
                "configuration_exception_files_observed":self.configuration_exception_files_observed,
                "dependency_roots_excluded":self.dependency_roots_excluded,
                "git_safety_status":"not_evaluated"
            },
            "languages":languages,
            "build_roots":build_roots,
            "declared_versions":self.declared_versions,
            "manifest_sha256":self.manifest_sha256,
            "lockfiles":self.lockfiles,
            "checker_configurations":self.checker_configurations,
            "native_tool_candidates":self.native_tool_candidates,
            "source_set_candidates":source_set_candidates,
            "blocked_paths":self.blocked_paths,
            "unknown_conditions":self.unknown_conditions,
        })
    }
}

/// 遍历当前项目的可读路径，不跟随符号链接，也不调用项目命令。
pub fn discover<P: ObservationPort>(
    root: &Path,
    registry: &LegacyRegistry,
    observation: &P,
) -> DiscoveryReport {
    let exact_config_files: BTreeSet<String> = registry
        .languages
        .iter()
        .flat_map(|language| language.linter_config_files.iter())
        .filter(|name| {
            !matches!(name.as_str(), "." | "..")
                && !name.contains('/')
                && !name.contains('\\')
                && !name.contains('*')
                && !name.contains('?')
                && !name.chars().any(char::is_control)
        })
        .cloned()
        .collect();
    let mut report = empty_report(root);
    if !matches!(observation.classify(root), Ok(ObservedPathKind::Directory)) {
        report.observation_complete = false;
        report.blocked_paths.push(".".into());
        return report;
    }
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let mut children = match observation.children(&directory) {
            Ok(children) => children,
            Err(_) => {
                block(&mut report, root, &directory);
                continue;
            }
        };
        children.sort();
        for child in children {
            let Some(name) = child.file_name().and_then(|name| name.to_str()) else {
                block(&mut report, root, &child);
                continue;
            };
            let configuration_exception = name.starts_with('.')
                && (exact_config_files.contains(name)
                    || ESLINT_CONFIG_NAMES.contains(&name)
                    || matches!(name, ".ruff.toml" | ".pre-commit-config.yaml"));
            if matches!(name, "node_modules" | ".mvn") {
                match observation.classify(&child) {
                    Ok(kind) => {
                        if let Some(relative) = relative_path(root, &child) {
                            report.tool_hint_paths.insert(relative);
                        }
                        if name == "node_modules" && kind == ObservedPathKind::Directory {
                            report.dependency_roots_excluded += 1;
                        } else if kind != ObservedPathKind::Directory {
                            block(&mut report, root, &child);
                        }
                    }
                    Err(_) => block(&mut report, root, &child),
                }
                if name == "node_modules" {
                    continue;
                }
            }
            if name.starts_with('.') && !configuration_exception {
                report.dot_prefix_roots_excluded += 1;
                continue;
            }
            if report.observed_entries >= MAX_ENTRIES {
                report.observation_complete = false;
                report.unknown_conditions.push("entry_limit_reached".into());
                return report;
            }
            report.observed_entries += 1;
            let Some(relative) = relative_path(root, &child) else {
                block(&mut report, root, &child);
                continue;
            };
            if managed_workspace_artifact(&relative) {
                continue;
            }
            match observation.classify(&child) {
                Ok(ObservedPathKind::Directory) if configuration_exception => {
                    report.dot_prefix_roots_excluded += 1;
                }
                Ok(ObservedPathKind::Directory) => pending.push(child),
                Ok(ObservedPathKind::File) => {
                    if name == "mvnw" {
                        report.tool_hint_paths.insert(relative.clone());
                    }
                    if configuration_exception {
                        report.configuration_exception_files_observed += 1;
                    }
                    observe_file(
                        &mut report,
                        registry,
                        observation,
                        &child,
                        &relative,
                        &exact_config_files,
                    )
                }
                Ok(ObservedPathKind::Symlink | ObservedPathKind::Other) | Err(_) => {
                    if name == "mvnw" {
                        report.tool_hint_paths.insert(relative.clone());
                    }
                    block(&mut report, root, &child);
                }
            }
        }
    }
    if !report.build_roots.is_empty() {
        for (build_root, manifests) in &report.build_roots {
            if let Some(manifest) = manifests.iter().find(|name| name.ends_with("pom.xml")) {
                match observation.read_bounded(&root.join(manifest), MAX_MANIFEST_BYTES) {
                    Ok(bytes) => report
                        .checker_configurations
                        .extend(inspect_maven_pom(&bytes, build_root, manifest)),
                    Err(_) => {
                        report.observation_complete = false;
                        report.blocked_paths.push(manifest.clone());
                        report
                            .checker_configurations
                            .extend(inspect_maven_unreadable(build_root, manifest));
                    }
                }
            } else if let Some(manifest) = manifests
                .iter()
                .find(|name| name.ends_with("build.gradle") || name.ends_with("build.gradle.kts"))
            {
                report
                    .checker_configurations
                    .extend(inspect_gradle_unknown(build_root, manifest));
            }
        }
        report
            .unknown_conditions
            .push("build_model_not_evaluated".into());
        if report.declared_versions.len() < report.build_roots.values().map(BTreeSet::len).sum() {
            report
                .unknown_conditions
                .push("some_manifest_metadata_not_parsed".into());
        }
    }
    inspect_python_ruff(root, observation, &mut report);
    inspect_python_cve_inputs(root, observation, &mut report);
    let eslint_candidates = report.eslint_config_files.clone();
    inspect_node_eslint(root, observation, &mut report, &eslint_candidates);
    inspect_native_tools(root, observation, &mut report);
    inspect_node_npm_audit(root, observation, &mut report);
    if report
        .languages
        .values()
        .any(|evidence| !evidence.source_files.is_empty())
    {
        report
            .unknown_conditions
            .push("source_sets_not_fully_classified".into());
    }
    if !report.languages.is_empty() {
        report
            .unknown_conditions
            .push("dialects_not_classified".into());
    }
    if !report.lockfiles.is_empty() {
        report
            .unknown_conditions
            .push("lock_contents_not_evaluated".into());
    }
    report.blocked_paths.sort();
    report.blocked_paths.dedup();
    report
}

/// 建立尚未观察任何路径的发现结果；完成度只适用于调用方选择的范围。
pub(crate) fn empty_report(root: &Path) -> DiscoveryReport {
    DiscoveryReport {
        root: root.to_string_lossy().into_owned(),
        observation_complete: true,
        languages: BTreeMap::new(),
        ambiguous_source_files: BTreeSet::new(),
        build_roots: BTreeMap::new(),
        declared_versions: BTreeMap::new(),
        maven_module_models: BTreeMap::new(),
        cargo_module_models: BTreeMap::new(),
        manifest_sha256: BTreeMap::new(),
        lock_sha256: BTreeMap::new(),
        checker_config_sha256: BTreeMap::new(),
        lockfiles: BTreeSet::new(),
        checker_configurations: Vec::new(),
        native_tool_candidates: Vec::new(),
        tool_hint_paths: BTreeSet::new(),
        ruff_config_files: BTreeSet::new(),
        eslint_config_files: BTreeSet::new(),
        source_set_candidates: BTreeSet::new(),
        blocked_paths: Vec::new(),
        unknown_conditions: Vec::new(),
        observed_entries: 0,
        dot_prefix_roots_excluded: 0,
        configuration_exception_files_observed: 0,
        dependency_roots_excluded: 0,
    }
}

/// 登记不可读取的 Ruff 候选，确保祖先配置不能越过该阻塞悄悄回退。
pub(crate) fn record_unavailable_ruff_config(report: &mut DiscoveryReport, relative: &str) {
    report.ruff_config_files.insert(relative.into());
}

fn managed_workspace_artifact(relative: &str) -> bool {
    if relative == "AGENTS.md" {
        return true;
    }
    let Some(within) = relative.strip_prefix(".codeguard/") else {
        return false;
    };
    if matches!(
        within,
        "README.md" | "workspace.json" | "project.json" | "module-graph.json" | "architecture.md"
    ) {
        return true;
    }
    let top = within.split('/').next().unwrap_or("");
    matches!(
        top,
        "findings" | "tasks" | "decisions" | "reports" | "runs" | "cache" | "worktrees" | "state"
    )
}

fn relative_path(root: &Path, path: &Path) -> Option<String> {
    let relative = path.strip_prefix(root).ok()?;
    if relative.as_os_str().is_empty() {
        return Some(".".into());
    }
    Some(relative.to_str()?.replace(std::path::MAIN_SEPARATOR, "/"))
}

fn block(report: &mut DiscoveryReport, root: &Path, path: &Path) {
    report.observation_complete = false;
    report
        .blocked_paths
        .push(relative_path(root, path).unwrap_or_else(|| "<non-utf8-or-outside-root>".into()));
}

pub(crate) fn observe_file<P: ObservationPort>(
    report: &mut DiscoveryReport,
    registry: &LegacyRegistry,
    observation: &P,
    path: &Path,
    relative: &str,
    exact_config_files: &BTreeSet<String>,
) {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        report.observation_complete = false;
        report.blocked_paths.push(relative.into());
        return;
    };
    if LOCK_NAMES.contains(&name) || is_standard_python_lock_name(name) {
        report.lockfiles.insert(relative.into());
        match observation.read_bounded(path, 8 * 1024 * 1024) {
            Ok(bytes) => {
                report
                    .lock_sha256
                    .insert(relative.into(), format!("{:x}", Sha256::digest(&bytes)));
            }
            Err(_) => {
                report.observation_complete = false;
                report.blocked_paths.push(relative.into());
            }
        }
    }
    let ruff_config = matches!(
        name,
        ".ruff.toml" | "ruff.toml" | "pyproject.toml" | ".pre-commit-config.yaml"
    );
    if ruff_config {
        report.ruff_config_files.insert(relative.into());
    }
    let eslint_config = ESLINT_CONFIG_NAMES.contains(&name);
    if eslint_config {
        report.eslint_config_files.insert(relative.into());
    }
    if ruff_config || eslint_config || exact_config_files.contains(name) {
        match observation.read_bounded(path, MAX_MANIFEST_BYTES) {
            Ok(bytes) => {
                report
                    .checker_config_sha256
                    .insert(relative.into(), format!("{:x}", Sha256::digest(&bytes)));
            }
            Err(_) => {
                report.observation_complete = false;
                report.blocked_paths.push(relative.into());
            }
        }
    }
    if name.ends_with(".sc") {
        report.ambiguous_source_files.insert(relative.into());
        report
            .unknown_conditions
            .push(format!("ambiguous_language_suffix:{relative}"));
        return;
    }
    if name.ends_with(".m") {
        match observation.read_bounded(path, 1024 * 1024) {
            Ok(source) if crate::source_language_hint::has_objc_marker(&source) => {}
            Ok(_) => {
                report.ambiguous_source_files.insert(relative.into());
                report
                    .unknown_conditions
                    .push(format!("ambiguous_language_suffix:{relative}"));
                return;
            }
            Err(_) => {
                report.observation_complete = false;
                report.blocked_paths.push(relative.into());
                report.ambiguous_source_files.insert(relative.into());
                return;
            }
        }
    }
    for language in &registry.languages {
        if language
            .extensions
            .iter()
            .any(|extension| name.ends_with(extension))
        {
            report
                .languages
                .entry(language.id.clone())
                .or_default()
                .source_files
                .insert(relative.into());
            if language.id == "java" {
                observe_java_source_set(report, relative);
            }
        }
        if language.markers.iter().any(|marker| marker == name) {
            if !report.manifest_sha256.contains_key(relative) {
                match observation.read_bounded(path, MAX_MANIFEST_BYTES) {
                    Ok(bytes) => {
                        report
                            .manifest_sha256
                            .insert(relative.into(), format!("{:x}", Sha256::digest(&bytes)));
                        if name == "Cargo.toml" {
                            match CargoModuleModel::observe(&bytes) {
                                Ok(model) => {
                                    for reason in model.unresolved.iter().filter(|reason| {
                                        reason.starts_with("language_target_")
                                            || reason.starts_with("package_version_")
                                    }) {
                                        report
                                            .unknown_conditions
                                            .push(format!("cargo_{reason}:{relative}"));
                                    }
                                    if let Some(version) = &model.package_version {
                                        report
                                            .declared_versions
                                            .insert(relative.into(), version.clone());
                                    }
                                    report.cargo_module_models.insert(relative.into(), model);
                                }
                                Err(reason) => report
                                    .unknown_conditions
                                    .push(format!("{reason}:{relative}")),
                            }
                        }
                        if name == "pom.xml" {
                            match MavenModuleModel::observe(&bytes) {
                                Ok(model) => {
                                    for reason in model.unresolved.iter().filter(|reason| {
                                        reason.starts_with("language_target_")
                                            || reason.starts_with("package_version_")
                                    }) {
                                        report
                                            .unknown_conditions
                                            .push(format!("maven_{reason}:{relative}"));
                                    }
                                    if let Some(version) = &model.package_version {
                                        report
                                            .declared_versions
                                            .insert(relative.into(), version.clone());
                                    }
                                    report.maven_module_models.insert(relative.into(), model);
                                }
                                Err(reason) => report
                                    .unknown_conditions
                                    .push(format!("{reason}:{relative}")),
                            }
                        }
                    }
                    Err(_) => {
                        report.observation_complete = false;
                        report.blocked_paths.push(relative.into());
                    }
                }
            }
            report
                .languages
                .entry(language.id.clone())
                .or_default()
                .manifests
                .insert(relative.into());
            let parent = Path::new(relative)
                .parent()
                .and_then(|parent| parent.to_str())
                .unwrap_or("");
            let parent = if parent.is_empty() { "." } else { parent };
            report
                .build_roots
                .entry(parent.into())
                .or_default()
                .insert(relative.into());
        }
    }
    if name == "package.json" {
        match observation.read_bounded(path, MAX_MANIFEST_BYTES) {
            Ok(bytes) => {
                report
                    .manifest_sha256
                    .insert(relative.into(), format!("{:x}", Sha256::digest(&bytes)));
                match parse_package_json(&bytes) {
                    Ok(declaration) => {
                        if let Some(version) = declaration.version {
                            report.declared_versions.insert(relative.into(), version);
                        } else {
                            report
                                .unknown_conditions
                                .push(format!("version_not_declared:{relative}"));
                        }
                    }
                    Err(_) => report
                        .unknown_conditions
                        .push(format!("manifest_parse_failed:{relative}")),
                }
            }
            Err(_) => {
                report.observation_complete = false;
                report.blocked_paths.push(relative.into());
            }
        }
    }
}

pub(crate) fn is_standard_python_lock_name(name: &str) -> bool {
    if name == "pylock.toml" {
        return true;
    }
    name.strip_prefix("pylock.")
        .and_then(|tail| tail.strip_suffix(".toml"))
        .is_some_and(|variant| !variant.is_empty() && !variant.contains('.'))
}

pub(crate) fn inspect_python_ruff<P: ObservationPort>(
    root: &Path,
    observation: &P,
    report: &mut DiscoveryReport,
) {
    let Some(python) = report.languages.get("python") else {
        return;
    };
    let targets: Vec<String> = if python.source_files.is_empty() {
        python.manifests.iter().cloned().collect()
    } else {
        python.source_files.iter().cloned().collect()
    };
    let mut inspected = BTreeMap::new();
    for candidate in &report.ruff_config_files {
        let build_root = relative_parent(candidate);
        let status = if candidate.ends_with(".pre-commit-config.yaml") {
            inspect_ruff_unknown(&build_root, candidate, "pre_commit_hooks_not_resolved")
        } else {
            match observation.read_bounded(&root.join(candidate), MAX_MANIFEST_BYTES) {
                Ok(bytes) => inspect_ruff_config(
                    &bytes,
                    &build_root,
                    candidate,
                    candidate.ends_with("pyproject.toml"),
                ),
                Err(_) => {
                    report.observation_complete = false;
                    report.blocked_paths.push(candidate.clone());
                    inspect_ruff_unknown(&build_root, candidate, "ruff_config_unreadable")
                }
            }
        };
        inspected.insert(candidate.clone(), status);
    }
    let mut selected = BTreeMap::new();
    for target in targets {
        let mut directory = relative_parent(&target);
        let mut chosen = None;
        loop {
            for name in [
                ".ruff.toml",
                "ruff.toml",
                "pyproject.toml",
                ".pre-commit-config.yaml",
            ] {
                let candidate = relative_child(&directory, name);
                if let Some(status) = inspected.get(&candidate) {
                    if status.configuration == "missing" {
                        continue;
                    }
                    chosen = Some(status.clone());
                    break;
                }
            }
            if chosen.is_some() || directory == "." {
                break;
            }
            directory = relative_parent(&directory);
        }
        let status =
            chosen.unwrap_or_else(|| inspect_ruff_missing(&nearest_python_root(&target, python)));
        selected.entry(status.build_root.clone()).or_insert(status);
    }
    report.checker_configurations.extend(selected.into_values());
}

fn nearest_python_root(target: &str, python: &LanguageEvidence) -> String {
    python
        .manifests
        .iter()
        .map(|manifest| relative_parent(manifest))
        .filter(|root| *root == "." || target.starts_with(&format!("{root}/")))
        .max_by_key(String::len)
        .unwrap_or_else(|| ".".into())
}

fn relative_parent(path: &str) -> String {
    Path::new(path)
        .parent()
        .and_then(|parent| parent.to_str())
        .filter(|parent| !parent.is_empty())
        .unwrap_or(".")
        .to_owned()
}

fn relative_child(parent: &str, name: &str) -> String {
    if parent == "." {
        name.into()
    } else {
        format!("{parent}/{name}")
    }
}

fn observe_java_source_set(report: &mut DiscoveryReport, relative: &str) {
    let parts: Vec<&str> = relative.split('/').collect();
    for (index, window) in parts.windows(3).enumerate() {
        if window[0] == "src" && window[2] == "java" && matches!(window[1], "main" | "test") {
            let source_root = parts[..index + 3].join("/");
            report
                .source_set_candidates
                .insert((source_root, "java".into(), window[1].into()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::discover;
    use codeguard_adapters::legacy_registry;
    use codeguard_core::{ObservationPort, ObservedPathKind};
    use std::collections::BTreeMap;
    use std::io;
    use std::path::{Path, PathBuf};

    struct FakeObservation {
        children: BTreeMap<PathBuf, Vec<PathBuf>>,
        unreadable: Option<PathBuf>,
    }

    impl ObservationPort for FakeObservation {
        fn classify(&self, path: &Path) -> io::Result<ObservedPathKind> {
            if self.children.contains_key(path) {
                Ok(ObservedPathKind::Directory)
            } else {
                Ok(ObservedPathKind::File)
            }
        }

        fn children(&self, directory: &Path) -> io::Result<Vec<PathBuf>> {
            if self.unreadable.as_deref() == Some(directory) {
                return Err(io::Error::new(io::ErrorKind::PermissionDenied, "fixture"));
            }
            Ok(self.children.get(directory).cloned().unwrap_or_default())
        }

        fn read_bounded(&self, path: &Path, _max_bytes: u64) -> io::Result<Vec<u8>> {
            if self.unreadable.as_deref() == Some(path) {
                return Err(io::Error::new(io::ErrorKind::PermissionDenied, "fixture"));
            }
            if path.file_name().is_some_and(|name| name == "pom.xml") {
                return Ok(b"<project/>".to_vec());
            }
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "no manifest fixture",
            ))
        }
    }

    fn fixture(unreadable: Option<&str>) -> FakeObservation {
        let mut children = BTreeMap::new();
        children.insert(
            PathBuf::from("project"),
            vec!["project/src", "project/pom.xml", "project/codeguard"]
                .into_iter()
                .map(PathBuf::from)
                .collect(),
        );
        children.insert(
            PathBuf::from("project/src"),
            vec![PathBuf::from("project/src/App.java")],
        );
        children.insert(
            PathBuf::from("project/codeguard"),
            vec![PathBuf::from("project/codeguard/custom.py")],
        );
        FakeObservation {
            children,
            unreadable: unreadable.map(PathBuf::from),
        }
    }

    #[test]
    fn keeps_evidence_from_source_and_codeguard_directory_without_claiming_build_model() {
        let report = discover(
            Path::new("project"),
            &legacy_registry().expect("fixed registry"),
            &fixture(None),
        );
        assert!(report.observation_complete);
        assert!(
            report.languages["java"]
                .source_files
                .contains("src/App.java")
        );
        assert!(report.languages["java"].manifests.contains("pom.xml"));
        assert!(
            report.languages["python"]
                .source_files
                .contains("codeguard/custom.py")
        );
        assert!(report.build_roots["."].contains("pom.xml"));
        assert!(
            report
                .unknown_conditions
                .contains(&"build_model_not_evaluated".into())
        );
    }

    #[test]
    fn unreadable_directory_returns_partial_evidence_and_incomplete_coverage() {
        let report = discover(
            Path::new("project"),
            &legacy_registry().expect("fixed registry"),
            &fixture(Some("project/src")),
        );
        assert!(!report.observation_complete);
        assert!(report.blocked_paths.contains(&"src".into()));
        assert!(report.languages["java"].manifests.contains("pom.xml"));
        assert!(
            report.languages["python"]
                .source_files
                .contains("codeguard/custom.py")
        );
    }

    #[test]
    fn unreadable_pom_is_unknown_environment_blocker_not_invalid_configuration() {
        let report = discover(
            Path::new("project"),
            &legacy_registry().expect("fixed registry"),
            &fixture(Some("project/pom.xml")),
        );
        assert!(!report.observation_complete);
        assert!(report.blocked_paths.contains(&"pom.xml".into()));
        let maven: Vec<_> = report
            .checker_configurations
            .iter()
            .filter(|checker| checker.checker_id.starts_with("java.maven."))
            .collect();
        assert_eq!(maven.len(), 7);
        assert!(maven.iter().all(|checker| {
            checker.configuration == "unknown" && checker.reason == "pom_unreadable"
        }));
    }
}
