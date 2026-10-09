//! `check` 报告的**形状决策**：哪些槽位出现、哪些字段被附加、按 `schema_version` 做的
//! 历史兼容修剪，以及 `lint_only` 的类别收窄。
//!
//! 这里的每一条规则都是「报告长什么样」的决定，与「检查怎么做」无关。把它从
//! `run_scoped` 里抽出来，是为了让调度代码不再同时承担协议形状与执行编排两件事——
//! 原先这段 220 行嵌在 3571 行函数中间，既难测也难改。
//!
//! 槽位存在性的**唯一事实源**是 [`crate::check_native_results::assemble`]；本模块只做
//! 后置的形状修剪，不要在这里新增语言专属的存在性规则。

use serde_json::Value;

/// 后处理所需的全部条件与可选结果。字段只收「决定报告形状」的东西，
/// 执行过程中的中间状态（选中的源集、工具路径等）不进这里。
pub(crate) struct ReportShapeInput<'a> {
    /// 选中的 C/C++ 文件路径；非空时附加 `c_family_comments` 槽位。
    pub c_paths_nonempty: bool,
    /// C/C++ 文档扫描结果；仅在 `c_paths` 非空时进入报告。
    pub c_family_comments: Value,
    /// 本轮请求了 Gradle CVE。
    pub gradle_cve_requested: bool,
    /// 本轮请求了 Gradle 模型/文档。
    pub gradle_requested: bool,
    /// Gradle 模型结果的槽位键（随构建器而变）。
    pub gradle_result_key: &'a str,
    /// Gradle CVE 结果。
    pub java_gradle_cve: Value,
    /// Gradle CVE 任务。
    pub gradle_cve_tasks: Value,
    /// Gradle 模型结果。
    pub java_gradle_model: Value,
    /// Gradle Javadoc 任务。
    pub gradle_javadoc_tasks: Value,
    /// Zig lint 结果。
    pub zig_lint: Value,
    /// Ruby lint 结果。
    pub ruby_lint: Value,
    /// Shell lint 结果。
    pub shell_lint: Value,
    /// 本轮只跑 lint 类别。
    pub lint_only: bool,
}

/// 按条件与 `schema_version` 修剪已装配好的报告。
///
/// 纯函数：只改 `report` 的形状，不执行任何检查、不读写磁盘。调用方保证
/// `report` 已由 [`crate::check_native_results::assemble`] 完成槽位装配。
pub(crate) fn finalize(report: &mut Value, input: ReportShapeInput<'_>) {
    // ---- 条件槽位：只在对应检查确实被请求时才出现 ----
    if input.c_paths_nonempty {
        report["native_results"]["c_family_comments"] = input.c_family_comments;
    }
    if input.gradle_cve_requested {
        report["native_results"]["java_gradle_cve"] = input.java_gradle_cve;
        report["gradle_cve_tasks"] = input.gradle_cve_tasks;
    }
    if input.gradle_requested {
        report["native_results"][input.gradle_result_key] = input.java_gradle_model;
    }
    if schema_in(
        report,
        &["0.67.0", "0.68.0", "0.69.0", "0.70.0", "0.71.0", "0.72.0"],
    ) {
        report["gradle_javadoc_tasks"] = input.gradle_javadoc_tasks;
    }

    // ---- 语言 lint 槽位：既可能是「本轮跑了」，也可能是历史版本就带的 ----
    if input.zig_lint.is_object()
        || schema_in(
            report,
            &[
                "0.47.0", "0.48.0", "0.49.0", "0.50.0", "0.51.0", "0.52.0", "0.53.0", "0.54.0",
                "0.55.0", "0.56.0", "0.57.0", "0.58.0", "0.59.0", "0.60.0", "0.61.0", "0.62.0",
                "0.63.0", "0.64.0", "0.65.0", "0.67.0", "0.68.0", "0.69.0", "0.70.0", "0.71.0",
                "0.72.0",
            ],
        )
    {
        report["native_results"]["zig_lint"] = input.zig_lint;
    }

    // ---- 历史封闭协议：旧版本的报告不携带 Kotlin/Swift 的新增原生字段 ----
    if !schema_in(
        report,
        &[
            "0.42.0", "0.43.0", "0.44.0", "0.45.0", "0.46.0", "0.47.0", "0.48.0", "0.49.0",
            "0.50.0", "0.51.0", "0.52.0", "0.53.0", "0.54.0", "0.55.0", "0.56.0", "0.57.0",
            "0.58.0", "0.59.0", "0.60.0", "0.61.0", "0.62.0", "0.63.0", "0.64.0", "0.65.0",
            "0.67.0", "0.68.0", "0.69.0", "0.70.0", "0.71.0", "0.72.0",
        ],
    ) {
        remove_slot(report, "kotlin_lint");
    }
    // 防御性冗余：kotlin_lint 只经 assemble 进入报告且该函数不产生 null 槽位，
    // 因此这条判断当前不可达。保留它是为了兜住绕过装配的直接写入。
    if report["native_results"]["kotlin_lint"].is_null() {
        remove_slot(report, "kotlin_lint");
    }
    if !schema_in(
        report,
        &[
            "0.43.0", "0.44.0", "0.45.0", "0.46.0", "0.47.0", "0.48.0", "0.49.0", "0.50.0",
            "0.51.0", "0.52.0", "0.53.0", "0.54.0", "0.55.0", "0.56.0", "0.57.0", "0.58.0",
            "0.59.0", "0.60.0", "0.61.0", "0.62.0", "0.63.0", "0.64.0", "0.65.0", "0.67.0",
            "0.68.0", "0.69.0", "0.70.0", "0.71.0", "0.72.0",
        ],
    ) {
        remove_slot(report, "swift_lint");
    }

    // ---- Ruby / Shell：仅新版本携带 ----
    if schema_in(
        report,
        &[
            "0.51.0", "0.52.0", "0.53.0", "0.54.0", "0.55.0", "0.56.0", "0.57.0", "0.58.0",
            "0.59.0", "0.60.0", "0.61.0", "0.62.0", "0.63.0", "0.64.0", "0.65.0", "0.67.0",
            "0.68.0", "0.69.0", "0.70.0", "0.71.0", "0.72.0",
        ],
    ) {
        report["native_results"]["ruby_lint"] = input.ruby_lint;
    }
    if schema_in(
        report,
        &[
            "0.52.0", "0.53.0", "0.54.0", "0.55.0", "0.56.0", "0.57.0", "0.58.0", "0.59.0",
            "0.60.0", "0.61.0", "0.62.0", "0.63.0", "0.64.0", "0.65.0", "0.67.0", "0.68.0",
            "0.69.0", "0.70.0", "0.71.0", "0.72.0",
        ],
    ) {
        report["native_results"]["shell_lint"] = input.shell_lint;
    }

    // ---- lint_only：类别收窄 ----
    if input.lint_only {
        report["requested_categories"] = serde_json::json!(["lint"]);
        // 全量历史同步保留其它类别事实，但本次简报不能指导执行独立 CVE/构建/注释任务。
        if report["next"]["repair_brief"]["checker_id"]
            .as_str()
            .is_some_and(|checker| {
                !matches!(
                    checker,
                    "python.ruff"
                        | "python.ruff.doctor"
                        | "rust.cargo_clippy"
                        | "node.eslint"
                        | "node.eslint.preparation"
                        | "java.maven.p3c"
                        | "go.vet"
                        | "shell.shellcheck"
                        | "syntax.native_confirmation"
                )
            })
        {
            report["next"] = Value::Null;
        }
    }
}

/// 报告的 `schema_version` 是否在给定集合内。
fn schema_in(report: &Value, versions: &[&str]) -> bool {
    report["schema_version"]
        .as_str()
        .is_some_and(|v| versions.contains(&v))
}

/// 从 `native_results` 中移除一个槽位。
fn remove_slot(report: &mut Value, key: &str) {
    if let Some(native) = report["native_results"].as_object_mut() {
        native.remove(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn input() -> ReportShapeInput<'static> {
        ReportShapeInput {
            c_paths_nonempty: false,
            c_family_comments: Value::Null,
            gradle_cve_requested: false,
            gradle_requested: false,
            gradle_result_key: "java_gradle_model",
            java_gradle_cve: Value::Null,
            gradle_cve_tasks: Value::Null,
            java_gradle_model: Value::Null,
            gradle_javadoc_tasks: Value::Null,
            zig_lint: Value::Null,
            ruby_lint: Value::Null,
            shell_lint: Value::Null,
            lint_only: false,
        }
    }

    fn report_with(version: &str) -> Value {
        json!({"schema_version": version, "native_results": {}})
    }

    /// 历史版本不携带 Kotlin 槽位：0.42 之前的报告形状保持封闭。
    #[test]
    fn kotlin_slot_is_removed_on_old_schema() {
        let mut r = report_with("0.38.0");
        r["native_results"]["kotlin_lint"] = json!({"status":"incomplete"});
        finalize(&mut r, input());
        assert!(r["native_results"].get("kotlin_lint").is_none());
    }

    /// 新版本保留 Kotlin 槽位：它承载新增原生字段。
    #[test]
    fn kotlin_slot_is_kept_on_new_schema() {
        let mut r = report_with("0.62.0");
        r["native_results"]["kotlin_lint"] = json!({"status":"incomplete"});
        finalize(&mut r, input());
        assert!(r["native_results"].get("kotlin_lint").is_some());
    }

    /// lint_only 收窄类别，并清除非 lint 的简报。
    #[test]
    fn lint_only_clears_non_lint_brief() {
        let mut r = report_with("0.62.0");
        r["next"] = json!({"repair_brief":{"checker_id":"java.maven.javadoc"}});
        let mut i = input();
        i.lint_only = true;
        finalize(&mut r, i);
        assert_eq!(r["requested_categories"], json!(["lint"]));
        assert!(r["next"].is_null());
    }

    /// lint_only 不清空 lint 类简报：它们仍需指导修复。
    #[test]
    fn lint_only_keeps_lint_brief() {
        let mut r = report_with("0.62.0");
        r["next"] = json!({"repair_brief":{"checker_id":"python.ruff"}});
        let mut i = input();
        i.lint_only = true;
        finalize(&mut r, i);
        assert!(r["next"].is_object());
    }

    /// 未请求 gradle 时，gradle 槽位不出现。
    #[test]
    fn gradle_slots_absent_without_request() {
        let mut r = report_with("0.72.0");
        finalize(&mut r, input());
        assert!(r["native_results"].get("java_gradle_cve").is_none());
        assert!(r.get("gradle_cve_tasks").is_none());
    }

    /// 请求 gradle CVE 时，结果与任务都出现。
    #[test]
    fn gradle_cve_slots_present_with_request() {
        let mut r = report_with("0.72.0");
        let mut i = input();
        i.gradle_cve_requested = true;
        i.java_gradle_cve = json!({"status":"incomplete"});
        i.gradle_cve_tasks = json!([]);
        finalize(&mut r, i);
        assert!(r["native_results"].get("java_gradle_cve").is_some());
        assert!(r.get("gradle_cve_tasks").is_some());
    }
}
