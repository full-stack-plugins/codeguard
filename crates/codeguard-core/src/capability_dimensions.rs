//! 能力登记维度，不代表真实适配器已验收。

/// 质量义务的六个独立类别；依赖治理与漏洞匹配分别留证。
pub const CHECK_CATEGORIES: [&str; 6] = [
    "lint",
    "comments",
    "dependencies",
    "cve",
    "security",
    "build",
];

/// 当前已定义的静态检测族。登记检测族不表示已有可执行适配器。
pub const CHECK_KINDS: [(&str, &str); 28] = [
    ("lint.syntax", "lint"),
    ("lint.style", "lint"),
    ("lint.type", "lint"),
    ("lint.dataflow", "lint"),
    ("lint.complexity", "lint"),
    ("lint.duplication", "lint"),
    ("lint.dead_code", "lint"),
    ("lint.architecture", "lint"),
    ("lint.api_compatibility", "lint"),
    ("lint.schema", "lint"),
    ("comments.api_docs", "comments"),
    ("comments.comment_policy", "comments"),
    ("comments.doc_links", "comments"),
    ("dependencies.graph", "dependencies"),
    ("dependencies.versions", "dependencies"),
    ("dependencies.licenses", "dependencies"),
    ("dependencies.sbom", "dependencies"),
    ("dependencies.provenance", "dependencies"),
    ("cve.advisory", "cve"),
    ("security.sast", "security"),
    ("security.secrets", "security"),
    ("security.iac", "security"),
    ("security.container", "security"),
    ("security.config", "security"),
    ("security.policy", "security"),
    ("build.compile", "build"),
    ("build.reproducibility", "build"),
    ("build.manifest", "build"),
];

/// 验证检测族与类别的精确对应关系。
#[must_use]
pub fn check_kind_belongs_to_category(check_kind: &str, category: &str) -> bool {
    CHECK_KINDS.iter().any(|(known_kind, known_category)| {
        *known_kind == check_kind && *known_category == category
    })
}

/// 首批候选平台，仅用于登记维度。
pub const CANDIDATE_PLATFORMS: [&str; 5] = [
    "macos_arm64",
    "macos_x86_64",
    "linux_x86_64",
    "linux_aarch64",
    "windows_x86_64",
];
