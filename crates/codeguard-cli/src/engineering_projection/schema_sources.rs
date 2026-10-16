// 已审历史Schema的编译期白名单；仅使用内存，不接受候选路径或网络资源。
pub(super) fn source(name: &str) -> Option<&'static [u8]> {
    match name {
        "c-family-comments-feedback-v0.5.schema.json" => Some(include_bytes!(
            "../../../../schemas/c-family-comments-feedback-v0.5.schema.json"
        )),
        "c-family-comments-feedback-v0.9.schema.json" => Some(include_bytes!(
            "../../../../schemas/c-family-comments-feedback-v0.9.schema.json"
        )),
        "c-family-documentation-scan-v0.1.schema.json" => Some(include_bytes!(
            "../../../../schemas/c-family-documentation-scan-v0.1.schema.json"
        )),
        "c-family-documentation-scans-v0.1.schema.json" => Some(include_bytes!(
            "../../../../schemas/c-family-documentation-scans-v0.1.schema.json"
        )),
        "cfquery-candidate-row-v0.1.schema.json" => Some(include_bytes!(
            "../../../../schemas/cfquery-candidate-row-v0.1.schema.json"
        )),
        "cfquery-structure-v0.1.schema.json" => Some(include_bytes!(
            "../../../../schemas/cfquery-structure-v0.1.schema.json"
        )),
        "check-aborted-v0.21.schema.json" => Some(include_bytes!(
            "../../../../schemas/check-aborted-v0.21.schema.json"
        )),
        "check-aborted-v0.22.schema.json" => Some(include_bytes!(
            "../../../../schemas/check-aborted-v0.22.schema.json"
        )),
        "check-feedback-v0.38.schema.json" => Some(include_bytes!(
            "../../../../schemas/check-feedback-v0.38.schema.json"
        )),
        "check-feedback-v0.39.schema.json" => Some(include_bytes!(
            "../../../../schemas/check-feedback-v0.39.schema.json"
        )),
        "check-feedback-v0.40.schema.json" => Some(include_bytes!(
            "../../../../schemas/check-feedback-v0.40.schema.json"
        )),
        "check-feedback-v0.41.schema.json" => Some(include_bytes!(
            "../../../../schemas/check-feedback-v0.41.schema.json"
        )),
        "check-feedback-v0.42.schema.json" => Some(include_bytes!(
            "../../../../schemas/check-feedback-v0.42.schema.json"
        )),
        "check-feedback-v0.43.schema.json" => Some(include_bytes!(
            "../../../../schemas/check-feedback-v0.43.schema.json"
        )),
        "check-feedback-v0.44.schema.json" => Some(include_bytes!(
            "../../../../schemas/check-feedback-v0.44.schema.json"
        )),
        "check-feedback-v0.45.schema.json" => Some(include_bytes!(
            "../../../../schemas/check-feedback-v0.45.schema.json"
        )),
        "check-feedback-v0.46.schema.json" => Some(include_bytes!(
            "../../../../schemas/check-feedback-v0.46.schema.json"
        )),
        "check-feedback-v0.47.schema.json" => Some(include_bytes!(
            "../../../../schemas/check-feedback-v0.47.schema.json"
        )),
        "check-feedback-v0.48.schema.json" => Some(include_bytes!(
            "../../../../schemas/check-feedback-v0.48.schema.json"
        )),
        "check-feedback-v0.49.schema.json" => Some(include_bytes!(
            "../../../../schemas/check-feedback-v0.49.schema.json"
        )),
        "check-feedback-v0.50.schema.json" => Some(include_bytes!(
            "../../../../schemas/check-feedback-v0.50.schema.json"
        )),
        "check-feedback-v0.51.schema.json" => Some(include_bytes!(
            "../../../../schemas/check-feedback-v0.51.schema.json"
        )),
        "check-feedback-v0.52.schema.json" => Some(include_bytes!(
            "../../../../schemas/check-feedback-v0.52.schema.json"
        )),
        "check-feedback-v0.53.schema.json" => Some(include_bytes!(
            "../../../../schemas/check-feedback-v0.53.schema.json"
        )),
        "check-feedback-v0.54.schema.json" => Some(include_bytes!(
            "../../../../schemas/check-feedback-v0.54.schema.json"
        )),
        "check-feedback-v0.55.schema.json" => Some(include_bytes!(
            "../../../../schemas/check-feedback-v0.55.schema.json"
        )),
        "check-feedback-v0.56.schema.json" => Some(include_bytes!(
            "../../../../schemas/check-feedback-v0.56.schema.json"
        )),
        "check-feedback-v0.57.schema.json" => Some(include_bytes!(
            "../../../../schemas/check-feedback-v0.57.schema.json"
        )),
        "check-feedback-v0.58.schema.json" => Some(include_bytes!(
            "../../../../schemas/check-feedback-v0.58.schema.json"
        )),
        "check-feedback-v0.59.schema.json" => Some(include_bytes!(
            "../../../../schemas/check-feedback-v0.59.schema.json"
        )),
        "check-feedback-v0.60.schema.json" => Some(include_bytes!(
            "../../../../schemas/check-feedback-v0.60.schema.json"
        )),
        "check-feedback-v0.61.schema.json" => Some(include_bytes!(
            "../../../../schemas/check-feedback-v0.61.schema.json"
        )),
        "check-feedback-v0.62.schema.json" => Some(include_bytes!(
            "../../../../schemas/check-feedback-v0.62.schema.json"
        )),
        "check-feedback-v0.63.schema.json" => Some(include_bytes!(
            "../../../../schemas/check-feedback-v0.63.schema.json"
        )),
        "check-feedback-v0.64.schema.json" => Some(include_bytes!(
            "../../../../schemas/check-feedback-v0.64.schema.json"
        )),
        "check-feedback-v0.65.schema.json" => Some(include_bytes!(
            "../../../../schemas/check-feedback-v0.65.schema.json"
        )),
        "check-feedback-v0.66.schema.json" => Some(include_bytes!(
            "../../../../schemas/check-feedback-v0.66.schema.json"
        )),
        "check-feedback-v0.67.schema.json" => Some(include_bytes!(
            "../../../../schemas/check-feedback-v0.67.schema.json"
        )),
        "check-feedback-v0.68.schema.json" => Some(include_bytes!(
            "../../../../schemas/check-feedback-v0.68.schema.json"
        )),
        "check-feedback-v0.69.schema.json" => Some(include_bytes!(
            "../../../../schemas/check-feedback-v0.69.schema.json"
        )),
        "check-feedback-v0.70.schema.json" => Some(include_bytes!(
            "../../../../schemas/check-feedback-v0.70.schema.json"
        )),
        "check-feedback-v0.71.schema.json" => Some(include_bytes!(
            "../../../../schemas/check-feedback-v0.71.schema.json"
        )),
        "check-feedback-v0.72.schema.json" => Some(include_bytes!(
            "../../../../schemas/check-feedback-v0.72.schema.json"
        )),
        "checkstyle-repair-brief-preview-v0.25.schema.json" => Some(include_bytes!(
            "../../../../schemas/checkstyle-repair-brief-preview-v0.25.schema.json"
        )),
        "clang-function-documentation-structure-v0.1.schema.json" => Some(include_bytes!(
            "../../../../schemas/clang-function-documentation-structure-v0.1.schema.json"
        )),
        "erlang-forms-scan-v0.2.schema.json" => Some(include_bytes!(
            "../../../../schemas/erlang-forms-scan-v0.2.schema.json"
        )),
        "gradle-dependency-check-probe-v0.2.schema.json" => Some(include_bytes!(
            "../../../../schemas/gradle-dependency-check-probe-v0.2.schema.json"
        )),
        "gradle-javadoc-probe-v0.1.schema.json" => Some(include_bytes!(
            "../../../../schemas/gradle-javadoc-probe-v0.1.schema.json"
        )),
        "gradle-javadoc-probe-v0.2.schema.json" => Some(include_bytes!(
            "../../../../schemas/gradle-javadoc-probe-v0.2.schema.json"
        )),
        "gradle-model-probe-v0.1.schema.json" => Some(include_bytes!(
            "../../../../schemas/gradle-model-probe-v0.1.schema.json"
        )),
        "grammar-probe-v0.3.schema.json" => Some(include_bytes!(
            "../../../../schemas/grammar-probe-v0.3.schema.json"
        )),
        "grammar-probe-v0.7.schema.json" => Some(include_bytes!(
            "../../../../schemas/grammar-probe-v0.7.schema.json"
        )),
        "grammar-probe-v0.8.schema.json" => Some(include_bytes!(
            "../../../../schemas/grammar-probe-v0.8.schema.json"
        )),
        "java-javadoc-local-feedback-v0.2.schema.json" => Some(include_bytes!(
            "../../../../schemas/java-javadoc-local-feedback-v0.2.schema.json"
        )),
        "java-javadoc-project-probe-v0.4.schema.json" => Some(include_bytes!(
            "../../../../schemas/java-javadoc-project-probe-v0.4.schema.json"
        )),
        "java-javadoc-project-probe-v0.5.schema.json" => Some(include_bytes!(
            "../../../../schemas/java-javadoc-project-probe-v0.5.schema.json"
        )),
        "javadoc-repair-brief-preview-v0.4.schema.json" => Some(include_bytes!(
            "../../../../schemas/javadoc-repair-brief-preview-v0.4.schema.json"
        )),
        "javascript-mode-observation-v0.1.schema.json" => Some(include_bytes!(
            "../../../../schemas/javascript-mode-observation-v0.1.schema.json"
        )),
        "kotlin-compile-scan-v0.1.schema.json" => Some(include_bytes!(
            "../../../../schemas/kotlin-compile-scan-v0.1.schema.json"
        )),
        "kotlin-compile-scan-v0.2.schema.json" => Some(include_bytes!(
            "../../../../schemas/kotlin-compile-scan-v0.2.schema.json"
        )),
        "kotlin-lint-feedback-v0.1.schema.json" => Some(include_bytes!(
            "../../../../schemas/kotlin-lint-feedback-v0.1.schema.json"
        )),
        "maven-javadoc-multifile-probe-v0.2.schema.json" => Some(include_bytes!(
            "../../../../schemas/maven-javadoc-multifile-probe-v0.2.schema.json"
        )),
        "maven-javadoc-repair-brief-preview-v0.2.schema.json" => Some(include_bytes!(
            "../../../../schemas/maven-javadoc-repair-brief-preview-v0.2.schema.json"
        )),
        "maven-javadoc-repair-brief-preview-v0.3.schema.json" => Some(include_bytes!(
            "../../../../schemas/maven-javadoc-repair-brief-preview-v0.3.schema.json"
        )),
        "npm-preparation-repair-brief-preview-v0.1.schema.json" => Some(include_bytes!(
            "../../../../schemas/npm-preparation-repair-brief-preview-v0.1.schema.json"
        )),
        "p3c-configuration-repair-brief-preview-v0.1.schema.json" => Some(include_bytes!(
            "../../../../schemas/p3c-configuration-repair-brief-preview-v0.1.schema.json"
        )),
        "repair-brief-preview-v0.10.schema.json" => Some(include_bytes!(
            "../../../../schemas/repair-brief-preview-v0.10.schema.json"
        )),
        "repair-brief-preview-v0.11.schema.json" => Some(include_bytes!(
            "../../../../schemas/repair-brief-preview-v0.11.schema.json"
        )),
        "repair-brief-preview-v0.12.schema.json" => Some(include_bytes!(
            "../../../../schemas/repair-brief-preview-v0.12.schema.json"
        )),
        "repair-brief-preview-v0.13.schema.json" => Some(include_bytes!(
            "../../../../schemas/repair-brief-preview-v0.13.schema.json"
        )),
        "repair-brief-preview-v0.14.schema.json" => Some(include_bytes!(
            "../../../../schemas/repair-brief-preview-v0.14.schema.json"
        )),
        "repair-brief-preview-v0.15.schema.json" => Some(include_bytes!(
            "../../../../schemas/repair-brief-preview-v0.15.schema.json"
        )),
        "repair-brief-preview-v0.17.schema.json" => Some(include_bytes!(
            "../../../../schemas/repair-brief-preview-v0.17.schema.json"
        )),
        "repair-brief-preview-v0.20.schema.json" => Some(include_bytes!(
            "../../../../schemas/repair-brief-preview-v0.20.schema.json"
        )),
        "repair-brief-preview-v0.21.schema.json" => Some(include_bytes!(
            "../../../../schemas/repair-brief-preview-v0.21.schema.json"
        )),
        "repair-brief-preview-v0.22.schema.json" => Some(include_bytes!(
            "../../../../schemas/repair-brief-preview-v0.22.schema.json"
        )),
        "repair-brief-preview-v0.23.schema.json" => Some(include_bytes!(
            "../../../../schemas/repair-brief-preview-v0.23.schema.json"
        )),
        "repair-brief-preview-v0.24.schema.json" => Some(include_bytes!(
            "../../../../schemas/repair-brief-preview-v0.24.schema.json"
        )),
        "repair-brief-preview-v0.27.schema.json" => Some(include_bytes!(
            "../../../../schemas/repair-brief-preview-v0.27.schema.json"
        )),
        "repair-brief-preview-v0.30.schema.json" => Some(include_bytes!(
            "../../../../schemas/repair-brief-preview-v0.30.schema.json"
        )),
        "repair-brief-preview-v0.33.schema.json" => Some(include_bytes!(
            "../../../../schemas/repair-brief-preview-v0.33.schema.json"
        )),
        "repair-brief-preview-v0.4.schema.json" => Some(include_bytes!(
            "../../../../schemas/repair-brief-preview-v0.4.schema.json"
        )),
        "repair-brief-preview-v0.5.schema.json" => Some(include_bytes!(
            "../../../../schemas/repair-brief-preview-v0.5.schema.json"
        )),
        "repair-brief-preview-v0.6.schema.json" => Some(include_bytes!(
            "../../../../schemas/repair-brief-preview-v0.6.schema.json"
        )),
        "repair-brief-preview-v0.7.schema.json" => Some(include_bytes!(
            "../../../../schemas/repair-brief-preview-v0.7.schema.json"
        )),
        "repair-brief-preview-v0.8.schema.json" => Some(include_bytes!(
            "../../../../schemas/repair-brief-preview-v0.8.schema.json"
        )),
        "repair-brief-preview-v0.9.schema.json" => Some(include_bytes!(
            "../../../../schemas/repair-brief-preview-v0.9.schema.json"
        )),
        "repair-brief-preview.schema.json" => Some(include_bytes!(
            "../../../../schemas/repair-brief-preview.schema.json"
        )),
        "ruby-lint-feedback-v0.1.schema.json" => Some(include_bytes!(
            "../../../../schemas/ruby-lint-feedback-v0.1.schema.json"
        )),
        "ruby-syntax-scan-v0.1.schema.json" => Some(include_bytes!(
            "../../../../schemas/ruby-syntax-scan-v0.1.schema.json"
        )),
        "ruby-syntax-scan-v0.2.schema.json" => Some(include_bytes!(
            "../../../../schemas/ruby-syntax-scan-v0.2.schema.json"
        )),
        "rust-clippy-repair-brief-preview-v0.1.schema.json" => Some(include_bytes!(
            "../../../../schemas/rust-clippy-repair-brief-preview-v0.1.schema.json"
        )),
        "shell-native-scan-v0.1.schema.json" => Some(include_bytes!(
            "../../../../schemas/shell-native-scan-v0.1.schema.json"
        )),
        "swift-lint-feedback-v0.1.schema.json" => Some(include_bytes!(
            "../../../../schemas/swift-lint-feedback-v0.1.schema.json"
        )),
        "swift-parse-scan-v0.1.schema.json" => Some(include_bytes!(
            "../../../../schemas/swift-parse-scan-v0.1.schema.json"
        )),
        "swift-parse-scan-v0.2.schema.json" => Some(include_bytes!(
            "../../../../schemas/swift-parse-scan-v0.2.schema.json"
        )),
        "syntax-lint-feedback-v0.2.schema.json" => Some(include_bytes!(
            "../../../../schemas/syntax-lint-feedback-v0.2.schema.json"
        )),
        "syntax-task-recheck-v0.10.schema.json" => Some(include_bytes!(
            "../../../../schemas/syntax-task-recheck-v0.10.schema.json"
        )),
        "syntax-task-recheck-v0.2.schema.json" => Some(include_bytes!(
            "../../../../schemas/syntax-task-recheck-v0.2.schema.json"
        )),
        "syntax-task-recheck-v0.4.schema.json" => Some(include_bytes!(
            "../../../../schemas/syntax-task-recheck-v0.4.schema.json"
        )),
        "syntax-task-recheck-v0.9.schema.json" => Some(include_bytes!(
            "../../../../schemas/syntax-task-recheck-v0.9.schema.json"
        )),
        "syntax-task-recheck.schema.json" => Some(include_bytes!(
            "../../../../schemas/syntax-task-recheck.schema.json"
        )),
        "zig-ast-scan-v0.1.schema.json" => Some(include_bytes!(
            "../../../../schemas/zig-ast-scan-v0.1.schema.json"
        )),
        "zig-ast-scan-v0.2.schema.json" => Some(include_bytes!(
            "../../../../schemas/zig-ast-scan-v0.2.schema.json"
        )),
        "zig-lint-feedback-v0.1.schema.json" => Some(include_bytes!(
            "../../../../schemas/zig-lint-feedback-v0.1.schema.json"
        )),
        "zig-lint-feedback-v0.2.schema.json" => Some(include_bytes!(
            "../../../../schemas/zig-lint-feedback-v0.2.schema.json"
        )),
        "check-feedback-v0.73.schema.json" => Some(include_bytes!(
            "../../../../schemas/check-feedback-v0.73.schema.json"
        )),
        _ => None,
    }
}
pub(super) const IDS: &[(&str, &str)] = &[
    (
        "https://codeguard.dev/schemas/clang-function-documentation-structure-v0.1.schema.json",
        "clang-function-documentation-structure-v0.1.schema.json",
    ),
    (
        "https://codeguard.dev/schemas/gradle-model-probe-v0.1.schema.json",
        "gradle-model-probe-v0.1.schema.json",
    ),
    (
        "urn:codeguard:schema:c-family-comments-feedback:0.5.0",
        "c-family-comments-feedback-v0.5.schema.json",
    ),
    (
        "urn:codeguard:schema:c-family-comments-feedback:0.9.0",
        "c-family-comments-feedback-v0.9.schema.json",
    ),
    (
        "urn:codeguard:schema:c-family-documentation-scan:0.1.0",
        "c-family-documentation-scan-v0.1.schema.json",
    ),
    (
        "urn:codeguard:schema:c-family-documentation-scans:0.1.0",
        "c-family-documentation-scans-v0.1.schema.json",
    ),
    (
        "urn:codeguard:schema:check-aborted:0.21.0",
        "check-aborted-v0.21.schema.json",
    ),
    (
        "urn:codeguard:schema:check-aborted:0.22.0",
        "check-aborted-v0.22.schema.json",
    ),
    (
        "urn:codeguard:schema:check-feedback:0.38.0",
        "check-feedback-v0.38.schema.json",
    ),
    (
        "urn:codeguard:schema:check-feedback:0.39.0",
        "check-feedback-v0.39.schema.json",
    ),
    (
        "urn:codeguard:schema:check-feedback:0.40.0",
        "check-feedback-v0.40.schema.json",
    ),
    (
        "urn:codeguard:schema:check-feedback:0.41.0",
        "check-feedback-v0.41.schema.json",
    ),
    (
        "urn:codeguard:schema:check-feedback:0.42.0",
        "check-feedback-v0.42.schema.json",
    ),
    (
        "urn:codeguard:schema:check-feedback:0.43.0",
        "check-feedback-v0.43.schema.json",
    ),
    (
        "urn:codeguard:schema:check-feedback:0.44.0",
        "check-feedback-v0.44.schema.json",
    ),
    (
        "urn:codeguard:schema:check-feedback:0.45.0",
        "check-feedback-v0.45.schema.json",
    ),
    (
        "urn:codeguard:schema:check-feedback:0.46.0",
        "check-feedback-v0.46.schema.json",
    ),
    (
        "urn:codeguard:schema:check-feedback:0.47.0",
        "check-feedback-v0.47.schema.json",
    ),
    (
        "urn:codeguard:schema:check-feedback:0.48.0",
        "check-feedback-v0.48.schema.json",
    ),
    (
        "urn:codeguard:schema:check-feedback:0.49.0",
        "check-feedback-v0.49.schema.json",
    ),
    (
        "urn:codeguard:schema:check-feedback:0.50.0",
        "check-feedback-v0.50.schema.json",
    ),
    (
        "urn:codeguard:schema:check-feedback:0.51.0",
        "check-feedback-v0.51.schema.json",
    ),
    (
        "urn:codeguard:schema:check-feedback:0.52.0",
        "check-feedback-v0.55.schema.json",
    ),
    (
        "urn:codeguard:schema:check-feedback:0.56.0",
        "check-feedback-v0.56.schema.json",
    ),
    (
        "urn:codeguard:schema:check-feedback:0.57.0",
        "check-feedback-v0.57.schema.json",
    ),
    (
        "urn:codeguard:schema:check-feedback:0.58.0",
        "check-feedback-v0.58.schema.json",
    ),
    (
        "urn:codeguard:schema:check-feedback:0.59.0",
        "check-feedback-v0.59.schema.json",
    ),
    (
        "urn:codeguard:schema:check-feedback:0.60.0",
        "check-feedback-v0.60.schema.json",
    ),
    (
        "urn:codeguard:schema:check-feedback:0.61.0",
        "check-feedback-v0.61.schema.json",
    ),
    (
        "urn:codeguard:schema:check-feedback:0.62.0",
        "check-feedback-v0.62.schema.json",
    ),
    (
        "urn:codeguard:schema:check-feedback:0.63.0",
        "check-feedback-v0.63.schema.json",
    ),
    (
        "urn:codeguard:schema:check-feedback:0.64.0",
        "check-feedback-v0.64.schema.json",
    ),
    (
        "urn:codeguard:schema:check-feedback:0.65.0",
        "check-feedback-v0.65.schema.json",
    ),
    (
        "urn:codeguard:schema:check-feedback:0.66.0",
        "check-feedback-v0.66.schema.json",
    ),
    (
        "urn:codeguard:schema:check-feedback:0.67.0",
        "check-feedback-v0.67.schema.json",
    ),
    (
        "urn:codeguard:schema:check-feedback:0.68.0",
        "check-feedback-v0.68.schema.json",
    ),
    (
        "urn:codeguard:schema:check-feedback:0.69.0",
        "check-feedback-v0.69.schema.json",
    ),
    (
        "urn:codeguard:schema:check-feedback:0.70.0",
        "check-feedback-v0.70.schema.json",
    ),
    (
        "urn:codeguard:schema:check-feedback:0.71.0",
        "check-feedback-v0.71.schema.json",
    ),
    (
        "urn:codeguard:schema:check-feedback:0.72.0",
        "check-feedback-v0.72.schema.json",
    ),
    (
        "urn:codeguard:schema:checkstyle-repair-brief-preview-v0.25",
        "checkstyle-repair-brief-preview-v0.25.schema.json",
    ),
    (
        "urn:codeguard:schema:erlang-forms-scan:0.2.0",
        "erlang-forms-scan-v0.2.schema.json",
    ),
    (
        "urn:codeguard:schema:gradle-javadoc-probe:0.1.0",
        "gradle-javadoc-probe-v0.1.schema.json",
    ),
    (
        "urn:codeguard:schema:gradle-javadoc-probe:0.2.0",
        "gradle-javadoc-probe-v0.2.schema.json",
    ),
    (
        "urn:codeguard:schema:grammar-probe:0.3.0",
        "grammar-probe-v0.3.schema.json",
    ),
    (
        "urn:codeguard:schema:grammar-probe:0.7.0",
        "grammar-probe-v0.7.schema.json",
    ),
    (
        "urn:codeguard:schema:grammar-probe:0.8.0",
        "grammar-probe-v0.8.schema.json",
    ),
    (
        "urn:codeguard:schema:java-javadoc-local-feedback:0.2.0",
        "java-javadoc-local-feedback-v0.2.schema.json",
    ),
    (
        "urn:codeguard:schema:java-javadoc-project-probe:0.4.0",
        "java-javadoc-project-probe-v0.4.schema.json",
    ),
    (
        "urn:codeguard:schema:java-javadoc-project-probe:0.5.0",
        "java-javadoc-project-probe-v0.5.schema.json",
    ),
    (
        "urn:codeguard:schema:javadoc-repair-brief-preview:0.4.0",
        "javadoc-repair-brief-preview-v0.4.schema.json",
    ),
    (
        "urn:codeguard:schema:javascript-mode-observation:0.1.0",
        "javascript-mode-observation-v0.1.schema.json",
    ),
    (
        "urn:codeguard:schema:kotlin-compile-scan:0.1.0",
        "kotlin-compile-scan-v0.1.schema.json",
    ),
    (
        "urn:codeguard:schema:kotlin-compile-scan:0.2.0",
        "kotlin-compile-scan-v0.2.schema.json",
    ),
    (
        "urn:codeguard:schema:kotlin-lint-feedback:0.1.0",
        "kotlin-lint-feedback-v0.1.schema.json",
    ),
    (
        "urn:codeguard:schema:maven-javadoc-multifile-probe:0.2.0",
        "maven-javadoc-multifile-probe-v0.2.schema.json",
    ),
    (
        "urn:codeguard:schema:maven-javadoc-repair-brief-preview:0.2.0",
        "maven-javadoc-repair-brief-preview-v0.2.schema.json",
    ),
    (
        "urn:codeguard:schema:maven-javadoc-repair-brief-preview:0.3.0",
        "maven-javadoc-repair-brief-preview-v0.3.schema.json",
    ),
    (
        "urn:codeguard:schema:npm-preparation-repair-brief-preview:0.1.0",
        "npm-preparation-repair-brief-preview-v0.1.schema.json",
    ),
    (
        "urn:codeguard:schema:p3c-configuration-repair-brief-preview:0.1.0",
        "p3c-configuration-repair-brief-preview-v0.1.schema.json",
    ),
    (
        "urn:codeguard:schema:repair-brief-preview:0.10.0",
        "repair-brief-preview-v0.10.schema.json",
    ),
    (
        "urn:codeguard:schema:repair-brief-preview:0.11.0",
        "repair-brief-preview-v0.11.schema.json",
    ),
    (
        "urn:codeguard:schema:repair-brief-preview:0.12.0",
        "repair-brief-preview-v0.12.schema.json",
    ),
    (
        "urn:codeguard:schema:repair-brief-preview:0.13.0",
        "repair-brief-preview-v0.13.schema.json",
    ),
    (
        "urn:codeguard:schema:repair-brief-preview:0.14.0",
        "repair-brief-preview-v0.14.schema.json",
    ),
    (
        "urn:codeguard:schema:repair-brief-preview:0.15.0",
        "repair-brief-preview-v0.15.schema.json",
    ),
    (
        "urn:codeguard:schema:repair-brief-preview:0.17.0",
        "repair-brief-preview-v0.17.schema.json",
    ),
    (
        "urn:codeguard:schema:repair-brief-preview:0.20.0",
        "repair-brief-preview-v0.20.schema.json",
    ),
    (
        "urn:codeguard:schema:repair-brief-preview:0.21.0",
        "repair-brief-preview-v0.21.schema.json",
    ),
    (
        "urn:codeguard:schema:repair-brief-preview:0.22.0",
        "repair-brief-preview-v0.22.schema.json",
    ),
    (
        "urn:codeguard:schema:repair-brief-preview:0.23.0",
        "repair-brief-preview-v0.23.schema.json",
    ),
    (
        "urn:codeguard:schema:repair-brief-preview:0.24.0",
        "repair-brief-preview-v0.24.schema.json",
    ),
    (
        "urn:codeguard:schema:repair-brief-preview:0.27.0",
        "repair-brief-preview-v0.27.schema.json",
    ),
    (
        "urn:codeguard:schema:repair-brief-preview:0.3.0",
        "repair-brief-preview.schema.json",
    ),
    (
        "urn:codeguard:schema:repair-brief-preview:0.30.0",
        "repair-brief-preview-v0.30.schema.json",
    ),
    (
        "urn:codeguard:schema:repair-brief-preview:0.33.0",
        "repair-brief-preview-v0.33.schema.json",
    ),
    (
        "urn:codeguard:schema:repair-brief-preview:0.4.0",
        "repair-brief-preview-v0.4.schema.json",
    ),
    (
        "urn:codeguard:schema:repair-brief-preview:0.5.0",
        "repair-brief-preview-v0.5.schema.json",
    ),
    (
        "urn:codeguard:schema:repair-brief-preview:0.6.0",
        "repair-brief-preview-v0.6.schema.json",
    ),
    (
        "urn:codeguard:schema:repair-brief-preview:0.7.0",
        "repair-brief-preview-v0.7.schema.json",
    ),
    (
        "urn:codeguard:schema:repair-brief-preview:0.8.0",
        "repair-brief-preview-v0.8.schema.json",
    ),
    (
        "urn:codeguard:schema:repair-brief-preview:0.9.0",
        "repair-brief-preview-v0.9.schema.json",
    ),
    (
        "urn:codeguard:schema:ruby-lint-feedback:0.1.0",
        "ruby-lint-feedback-v0.1.schema.json",
    ),
    (
        "urn:codeguard:schema:ruby-syntax-scan:0.1.0",
        "ruby-syntax-scan-v0.1.schema.json",
    ),
    (
        "urn:codeguard:schema:ruby-syntax-scan:0.2.0",
        "ruby-syntax-scan-v0.2.schema.json",
    ),
    (
        "urn:codeguard:schema:rust-clippy-repair-brief-preview:0.1.0",
        "rust-clippy-repair-brief-preview-v0.1.schema.json",
    ),
    (
        "urn:codeguard:schema:shell-native-scan:0.1.0",
        "shell-native-scan-v0.1.schema.json",
    ),
    (
        "urn:codeguard:schema:swift-lint-feedback:0.1.0",
        "swift-lint-feedback-v0.1.schema.json",
    ),
    (
        "urn:codeguard:schema:swift-parse-scan:0.1.0",
        "swift-parse-scan-v0.1.schema.json",
    ),
    (
        "urn:codeguard:schema:swift-parse-scan:0.2.0",
        "swift-parse-scan-v0.2.schema.json",
    ),
    (
        "urn:codeguard:schema:syntax-task-recheck:0.1.0",
        "syntax-task-recheck.schema.json",
    ),
    (
        "urn:codeguard:schema:syntax-task-recheck:0.10.0",
        "syntax-task-recheck-v0.10.schema.json",
    ),
    (
        "urn:codeguard:schema:syntax-task-recheck:0.2.0",
        "syntax-task-recheck-v0.2.schema.json",
    ),
    (
        "urn:codeguard:schema:syntax-task-recheck:0.4.0",
        "syntax-task-recheck-v0.4.schema.json",
    ),
    (
        "urn:codeguard:schema:syntax-task-recheck:0.9.0",
        "syntax-task-recheck-v0.9.schema.json",
    ),
    (
        "urn:codeguard:schema:zig-ast-scan:0.1.0",
        "zig-ast-scan-v0.1.schema.json",
    ),
    (
        "urn:codeguard:schema:zig-ast-scan:0.2.0",
        "zig-ast-scan-v0.2.schema.json",
    ),
    (
        "urn:codeguard:schema:zig-lint-feedback:0.1.0",
        "zig-lint-feedback-v0.1.schema.json",
    ),
    (
        "urn:codeguard:schema:zig-lint-feedback:0.2.0",
        "zig-lint-feedback-v0.2.schema.json",
    ),
    (
        "urn:codeguard:syntax-lint-feedback:0.2",
        "syntax-lint-feedback-v0.2.schema.json",
    ),
    (
        "urn:codeguard:schema:check-feedback:0.73.0",
        "check-feedback-v0.73.schema.json",
    ),
];
