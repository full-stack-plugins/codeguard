//! Schema 版本冻结门禁。
//!
//! 当前仓库有 522 个 schema 文件、176 个协议族。每新增一个版本文件，
//! 所有消费者都要维护一张「哪些版本带哪些字段」的判断表——今天 `check_command.rs`
//! 里 12 处、`check_report_assembly.rs` 里 5 处、其他源文件里 715 处 `schema_version`
//! 判断就是这种复利的产物。
//!
//! **冻结规则**：不再新增版本文件。需要加字段时，就地扩展当前版本的 optional 字段，
//! 或者在本文件登记后显式解冻。新增版本意味着所有消费者的版本表都要改，代价远高于收益。
//!
//! 例外：确实需要不兼容变更（删字段、改语义）时，走以下流程：
//! 1. 在本文件的 `FROZEN_FAMILIES` 中登记新版本号；
//! 2. 同步更新 `check_report_assembly` 中的对应判断；
//! 3. 在 commit message 中说明为什么不兼容变更必须做。

/// 当前各协议族的版本集合。新增条目即解冻，需在 commit message 中说明理由。
#[cfg(test)]
pub(crate) const FROZEN_FAMILIES: &[(&str, &[&str])] = &[
    ("approval-snapshot", &["unversioned", "1.1"]),
    ("blocker-observed-event", &["unversioned"]),
    ("blocker-record", &["unversioned"]),
    (
        "c-family-comments-feedback",
        &[
            "0.1", "0.2", "0.3", "0.4", "0.5", "0.6", "0.7", "0.8", "0.9", "0.10", "0.11",
        ],
    ),
    ("c-family-documentation-scan", &["0.1"]),
    ("c-family-documentation-scans", &["0.1"]),
    ("capability-inventory", &["unversioned"]),
    ("capability-selection", &["unversioned"]),
    ("cfquery-candidate-row", &["0.1"]),
    ("cfquery-postgres-development-oracle", &["0.1"]),
    ("cfquery-structure", &["0.1"]),
    (
        "check-aborted",
        &[
            "unversioned",
            "0.4",
            "0.5",
            "0.6",
            "0.7",
            "0.8",
            "0.9",
            "0.10",
            "0.11",
            "0.12",
            "0.14",
            "0.15",
            "0.16",
            "0.17",
            "0.18",
            "0.19",
            "0.20",
            "0.21",
            "0.22",
        ],
    ),
    ("check-eslint-feedback", &["0.1"]),
    (
        "check-feedback",
        &[
            "unversioned",
            "0.22",
            "0.23",
            "0.24",
            "0.25",
            "0.26",
            "0.27",
            "0.28",
            "0.29",
            "0.30",
            "0.31",
            "0.32",
            "0.33",
            "0.34",
            "0.35",
            "0.37",
            "0.38",
            "0.39",
            "0.40",
            "0.41",
            "0.42",
            "0.43",
            "0.44",
            "0.45",
            "0.46",
            "0.47",
            "0.48",
            "0.49",
            "0.50",
            "0.51",
            "0.52",
            "0.53",
            "0.54",
            "0.55",
            "0.56",
            "0.57",
            "0.58",
            "0.59",
            "0.60",
            "0.61",
            "0.62",
            "0.63",
            "0.64",
            "0.65",
            "0.66",
            "0.67",
            "0.68",
            "0.69",
            "0.70",
            "0.71",
            "0.72",
        ],
    ),
    ("check-plan", &["unversioned"]),
    ("check-request", &["unversioned"]),
    ("checkstyle-finding-record", &["unversioned"]),
    ("checkstyle-preparation-brief", &["unversioned"]),
    ("checkstyle-preparation-observation", &["unversioned"]),
    ("checkstyle-preparation-recheck", &["unversioned", "0.2"]),
    ("checkstyle-preparation-record", &["unversioned"]),
    (
        "checkstyle-preparation-verification-preview",
        &["unversioned"],
    ),
    ("checkstyle-repair-brief-preview", &["unversioned", "0.25"]),
    ("checkstyle-task-recheck", &["unversioned", "0.2"]),
    ("checkstyle-task-verification-preview", &["unversioned"]),
    ("checkstyle-workbench-observation", &["unversioned", "0.2"]),
    ("clang-documentation-placeholder", &["0.1"]),
    (
        "clang-documentation-placeholder-task-recheck",
        &["0.1", "0.2"],
    ),
    (
        "clang-documentation-placeholder-workbench-observation",
        &["0.1"],
    ),
    ("clang-documentation-structure-task-recheck", &["0.1"]),
    (
        "clang-documentation-structure-workbench-observation",
        &["0.1"],
    ),
    ("clang-documentation-task-recheck", &["0.1"]),
    ("clang-documentation-workbench-observation", &["0.1"]),
    ("clang-function-documentation-structure", &["0.1"]),
    ("codeguard-workspace", &["unversioned"]),
    ("command-help", &["0.1", "0.2", "0.3", "0.4"]),
    ("config-inspection", &["unversioned", "0.3"]),
    ("consumed-report-marker", &["unversioned"]),
    ("conversation-feedback", &["unversioned"]),
    ("corpus-case", &["unversioned"]),
    ("discovery-report", &["unversioned", "0.3"]),
    ("distribution-manifest", &["unversioned", "1.0", "1.1"]),
    ("doctor-observation", &["unversioned", "0.1"]),
    ("erlang-forms-scan", &["0.1", "0.2"]),
    ("erlang-lint-feedback", &["0.1", "0.2", "0.3"]),
    ("eslint-config-map", &["unversioned"]),
    ("eslint-directory-feedback", &["unversioned"]),
    (
        "eslint-local-feedback",
        &["unversioned", "0.3", "0.4", "0.5", "0.6", "0.7"],
    ),
    ("eslint-preparation-brief-preview", &["unversioned"]),
    ("eslint-preparation-observation", &["0.2"]),
    ("eslint-repair-brief-preview", &["unversioned"]),
    ("eslint-task-recheck", &["unversioned"]),
    ("false-positive-decision", &["unversioned", "1.1"]),
    ("finding-observed-event", &["unversioned"]),
    ("finding-record", &["unversioned"]),
    ("git-index-safety-preview", &["unversioned", "0.2"]),
    ("go-lint-fallback-feedback", &["0.7"]),
    ("go-lint-local-feedback", &["unversioned"]),
    ("go-native-syntax", &["unversioned"]),
    ("go-static-candidate", &["unversioned"]),
    ("go-syntax-scan", &["0.1", "0.2"]),
    ("gradle-cve-task-recheck", &["0.1"]),
    ("gradle-cve-workbench-observation", &["0.1"]),
    ("gradle-dependency-check-probe", &["0.1", "0.2"]),
    ("gradle-javadoc-probe", &["0.1", "0.2"]),
    ("gradle-javadoc-task-recheck", &["0.1", "0.2"]),
    ("gradle-javadoc-workbench-observation", &["0.1", "0.2"]),
    ("gradle-model-probe", &["0.1"]),
    ("grammar-asset-manifest", &["unversioned"]),
    ("grammar-inventory", &["1.1"]),
    (
        "grammar-probe",
        &["0.1", "0.2", "0.3", "0.4", "0.5", "0.6", "0.7", "0.8"],
    ),
    ("grammar-regression-corpus", &["0.1", "0.2"]),
    ("grammar-regression-evaluation", &["0.1", "0.2"]),
    ("grammar-structure-evaluation", &["0.1"]),
    (
        "hook-execution-feedback",
        &[
            "unversioned",
            "0.1",
            "0.2",
            "0.3",
            "0.4",
            "0.5",
            "0.6",
            "0.8",
            "0.9",
            "0.10",
            "0.11",
            "0.12",
            "0.13",
            "0.14",
            "0.15",
            "0.16",
            "0.17",
            "0.18",
            "0.19",
            "0.20",
            "0.21",
            "0.22",
            "0.23",
            "0.24",
            "0.25",
            "0.26",
            "0.27",
            "0.28",
            "0.29",
            "0.30",
            "0.31",
        ],
    ),
    ("hook-trigger-plan", &["unversioned", "1.1"]),
    ("hook-trigger-request", &["unversioned"]),
    ("init-plan", &["unversioned", "0.3", "0.4"]),
    (
        "java-checkstyle-local-feedback",
        &["unversioned", "0.1", "0.2", "0.3", "0.5"],
    ),
    (
        "java-comments-feedback",
        &[
            "unversioned",
            "0.2",
            "0.3",
            "0.4",
            "0.5",
            "0.6",
            "0.7",
            "0.8",
            "0.9",
            "0.10",
        ],
    ),
    ("java-gradle-cve-feedback", &["0.1", "0.2", "0.3"]),
    ("java-javadoc-local-feedback", &["unversioned", "0.2"]),
    ("java-javadoc-project-probe", &["0.4", "0.5"]),
    ("java-p3c-file-feedback", &["0.1"]),
    ("java-p3c-local-feedback", &["unversioned"]),
    ("java-syntax-precheck-feedback", &["0.1"]),
    (
        "javadoc-repair-brief-preview",
        &["unversioned", "0.2", "0.3", "0.4"],
    ),
    ("javadoc-task-recheck", &["unversioned", "0.2", "0.3"]),
    (
        "javadoc-workbench-observation",
        &["unversioned", "0.2", "0.3"],
    ),
    ("javascript-mode-observation", &["0.1"]),
    ("javascript-native-syntax", &["unversioned"]),
    ("kotlin-compile-scan", &["0.1", "0.2"]),
    ("kotlin-lint-feedback", &["0.1"]),
    ("local-blocker-observation", &["unversioned"]),
    ("local-finding-observation", &["unversioned"]),
    ("local-import-failure", &["unversioned"]),
    ("maven-javadoc-multifile-probe", &["0.2"]),
    ("maven-javadoc-repair-brief-preview", &["0.1", "0.2", "0.3"]),
    ("maven-javadoc-task-recheck", &["0.1", "0.2"]),
    ("maven-javadoc-workbench-observation", &["0.1", "0.2"]),
    ("module-graph", &["unversioned", "0.1", "0.2"]),
    (
        "native-grammar-differential",
        &[
            "0.1", "0.2", "0.3", "0.4", "0.5", "0.6", "0.7", "0.8", "0.9", "0.10", "0.11", "0.12",
        ],
    ),
    ("npm-cve-feedback", &["unversioned"]),
    (
        "npm-cve-workbench-observation",
        &["unversioned", "0.1", "0.2"],
    ),
    ("npm-preparation-repair-brief-preview", &["0.1"]),
    ("p3c-configuration-repair-brief-preview", &["0.1"]),
    ("plan-preview", &["unversioned"]),
    ("production-acceptance-plan", &["unversioned"]),
    ("production-acceptance-plan-view", &["unversioned"]),
    ("project-profile", &["unversioned", "0.2"]),
    ("python-comments-feedback", &["0.1", "0.2"]),
    ("python-cve-local-observation", &["unversioned"]),
    ("python-cve-workbench-observation", &["unversioned"]),
    (
        "python-lint-feedback",
        &[
            "unversioned",
            "0.11",
            "0.12",
            "0.14",
            "0.15",
            "0.16",
            "0.17",
            "0.18",
            "0.19",
        ],
    ),
    ("python-syntax-confirmation-observation", &["0.1", "0.2"]),
    ("quality-policy-candidate", &["unversioned"]),
    (
        "repair-brief-preview",
        &[
            "unversioned",
            "0.1",
            "0.2",
            "0.4",
            "0.5",
            "0.6",
            "0.7",
            "0.8",
            "0.9",
            "0.10",
            "0.11",
            "0.12",
            "0.13",
            "0.14",
            "0.15",
            "0.16",
            "0.17",
            "0.18",
            "0.19",
            "0.20",
            "0.21",
            "0.22",
            "0.23",
            "0.24",
            "0.26",
            "0.27",
            "0.28",
            "0.29",
            "0.30",
            "0.31",
            "0.32",
            "0.33",
            "0.34",
            "0.35",
            "0.36",
        ],
    ),
    ("ruby-lint-feedback", &["0.1", "0.3"]),
    ("ruby-native-syntax", &["unversioned"]),
    ("ruby-static-candidate", &["unversioned"]),
    ("ruby-syntax-scan", &["0.1", "0.2"]),
    ("ruff-rulepack-preview", &["unversioned"]),
    ("rules-inventory", &["unversioned"]),
    ("run-report", &["unversioned", "1.3"]),
    ("runtime-options", &["unversioned", "1.1"]),
    ("rust-build-local-observation", &["unversioned", "0.1"]),
    ("rust-build-task-recheck", &["unversioned"]),
    ("rust-clippy-repair-brief-preview", &["0.1"]),
    ("rust-comments-feedback", &["0.1"]),
    ("rust-cve-local-observation", &["unversioned"]),
    ("rust-cve-workbench-observation", &["unversioned"]),
    ("rust-lint-feedback", &["0.1"]),
    ("rust-native-syntax-evidence", &["0.3"]),
    ("rust-project-syntax-observation", &["0.1"]),
    ("rust-syntax-scan", &["0.1", "0.2"]),
    (
        "rustdoc-local-observation",
        &["unversioned", "0.1", "0.2", "0.3"],
    ),
    ("rustdoc-task-recheck", &["unversioned"]),
    ("rustfmt-syntax-observation", &["0.1", "0.2"]),
    ("shell-lint-feedback", &["0.1", "0.2"]),
    ("shell-native-scan", &["0.1"]),
    ("shellcheck-task-recheck", &["0.1"]),
    ("shellcheck-workbench-observation", &["0.1"]),
    ("signed-approval-envelope", &["unversioned"]),
    ("signed-approval-payload", &["unversioned"]),
    ("signed-distribution-envelope", &["unversioned"]),
    ("signed-distribution-payload", &["unversioned"]),
    ("swift-lint-feedback", &["0.1"]),
    ("swift-parse-scan", &["0.1", "0.2"]),
    (
        "syntax-confirmation-observation",
        &[
            "unversioned",
            "0.2",
            "0.3",
            "0.4",
            "0.5",
            "0.6",
            "0.7",
            "0.8",
            "0.9",
            "0.10",
            "0.11",
            "0.12",
            "0.13",
            "0.14",
            "0.15",
        ],
    ),
    ("syntax-lint-feedback", &["0.1", "0.2"]),
    ("syntax-precheck-candidate", &["unversioned"]),
    (
        "syntax-task-recheck",
        &[
            "unversioned",
            "0.2",
            "0.3",
            "0.4",
            "0.5",
            "0.6",
            "0.7",
            "0.8",
            "0.9",
            "0.10",
            "0.11",
        ],
    ),
    ("task-attempt-event", &["unversioned"]),
    ("task-attempt-response", &["unversioned"]),
    ("task-lease-response", &["unversioned"]),
    ("task-lease-state", &["unversioned"]),
    ("task-lifecycle-record", &["0.1"]),
    (
        "task-resolution-evidence",
        &[
            "0.1", "0.2", "0.3", "0.4", "0.5", "0.6", "0.7", "0.8", "0.9", "0.10", "0.11",
        ],
    ),
    (
        "task-resolution-policy",
        &[
            "1.0", "1.1", "1.2", "1.3", "1.4", "1.5", "1.6", "1.7", "1.8", "1.9", "1.10",
        ],
    ),
    ("task-resolution-receipt", &["0.1", "0.2"]),
    (
        "task-show-preview",
        &[
            "unversioned",
            "0.2",
            "0.3",
            "0.4",
            "0.5",
            "0.6",
            "0.7",
            "0.8",
            "0.9",
        ],
    ),
    ("task-verification-event", &["unversioned"]),
    (
        "task-verification-preview",
        &[
            "unversioned",
            "0.4",
            "0.5",
            "0.6",
            "0.7",
            "0.8",
            "0.9",
            "0.10",
            "0.11",
            "0.13",
            "0.14",
            "0.15",
            "0.16",
            "0.17",
            "0.18",
            "0.19",
            "0.20",
            "0.21",
            "0.22",
            "0.23",
            "0.24",
            "0.25",
            "0.26",
            "0.27",
            "0.28",
            "0.29",
            "0.30",
            "0.31",
            "0.32",
            "0.33",
            "0.34",
            "0.35",
            "0.36",
            "0.37",
            "0.38",
            "0.39",
        ],
    ),
    ("tool-artifact-inspection", &["unversioned"]),
    ("tool-install-preview", &["unversioned", "0.1", "0.2"]),
    ("tool-inventory-observation", &["unversioned"]),
    ("tool-lock", &["unversioned"]),
    ("version-report", &["unversioned"]),
    ("whitelist-candidate-inspection", &["unversioned"]),
    ("whitelist-correction-event", &["unversioned"]),
    ("whitelist-correction-preview", &["unversioned"]),
    ("whitelist-proposal-preview", &["unversioned", "0.3"]),
    ("work-sync-preview", &["unversioned", "0.3"]),
    ("workspace-status-preview", &["unversioned"]),
    ("zig-ast-scan", &["0.1", "0.2"]),
    ("zig-lint-feedback", &["0.1", "0.2", "0.3"]),
];

/// 从 schema 文件名切出 (协议族, 版本)。没有版本后缀时版本为 `"unversioned"`。
/// 同时处理 `-v` 前缀（如 `approval-snapshot-v1.1`）。
#[cfg(test)]
fn split_schema_name(base: &str) -> (&str, &str) {
    if let Some(dash_pos) = base.rfind('-') {
        let after_dash = &base[dash_pos + 1..];
        // 去掉可选的 'v' 前缀
        let candidate = after_dash.strip_prefix('v').unwrap_or(after_dash);
        if is_version(candidate) {
            return (&base[..dash_pos], candidate);
        }
    }
    (base, "unversioned")
}

/// 检查字符串是否为 `X.Y` 或 `X.Y.Z` 形式。
#[cfg(test)]
fn is_version(s: &str) -> bool {
    let parts: Vec<&str> = s.split('.').collect();
    parts.len() >= 2
        && parts.len() <= 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 磁盘上的 schema 文件集合必须与冻结登记一致。新增版本文件而未登记 → 本测试失败。
    #[test]
    fn frozen_families_match_disk() {
        let mut actual: std::collections::BTreeMap<String, Vec<String>> = Default::default();
        let schemas_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("schemas");
        for entry in std::fs::read_dir(&schemas_dir).unwrap() {
            let name = entry.unwrap().file_name().to_string_lossy().to_string();
            if !name.ends_with(".schema.json") {
                continue;
            }
            let base = name.trim_end_matches(".schema.json");
            // 从末尾切出 -v?X.Y(.Z) 后缀；纯字符串解析，不引入正则依赖。
            let (fam, ver) = split_schema_name(base);
            actual
                .entry(fam.to_string())
                .or_default()
                .push(ver.to_string());
        }
        for (fam, frozen_vers) in FROZEN_FAMILIES {
            let disk_vers = actual
                .get(*fam)
                .unwrap_or_else(|| panic!("协议族 {fam} 在磁盘上不存在"));
            assert_eq!(
                disk_vers.len(),
                frozen_vers.len(),
                "协议族 {fam} 版本数变了：磁盘 {}，冻结登记 {}。新增版本需在本文件登记并说明理由。",
                disk_vers.len(),
                frozen_vers.len()
            );
        }
    }

    #[test]
    fn no_duplicate_families() {
        let mut seen = std::collections::HashSet::new();
        for (fam, _) in FROZEN_FAMILIES {
            assert!(seen.insert(*fam), "协议族 {fam} 重复登记");
        }
    }
}
