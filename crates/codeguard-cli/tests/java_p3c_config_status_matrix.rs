//! OpenSpec 6.2/15.4：Java 开发规范（P3C）配置状态与规则加载反例矩阵。
//!
//! 覆盖：禁用规则（skip=true → invalid）、management-only、profile 限定、
//! 已配置但含 profile 段（配置存在不等于生效）、正反例同探针结果相反、
//! 坏 XML、超时。所有状态必须区分 configured/missing/invalid/unknown，
//! 且 files[] 需携带底层检查器原因，不能只给聚合状态。

#![cfg(unix)]

use codeguard_cli::tool_identity::hash_bundle_tree;
use serde_json::Value;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

const CONFIGURED_POM: &str = include_str!("../../../tests/fixtures/p3c_native/pom.xml");

struct Project(PathBuf);

impl Project {
    fn new() -> Self {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir()
            .canonicalize()
            .expect("physical temp root")
            .join(format!("cg-p3c-config-matrix-{}-{id}", std::process::id()));
        fs::create_dir_all(root.join("src/main/java")).expect("source tree");
        fs::write(
            root.join("src/main/java/Bad_Name.java"),
            "class Bad_Name {}\n",
        )
        .expect("violating sentinel");
        fs::write(root.join("pom.xml"), CONFIGURED_POM).expect("configured POM");
        Self(root)
    }

    fn write_pom(&self, pom: &str) {
        fs::write(self.0.join("pom.xml"), pom).expect("POM variant");
    }

    fn check(&self, extra: &[&str]) -> Value {
        self.check_env(extra, None)
    }

    fn check_env(&self, extra: &[&str], timeout: Option<&str>) -> Value {
        let mut command = Command::new(env!("CARGO_BIN_EXE_codeguard"));
        command
            .args(["check", "all", self.0.to_str().unwrap(), "--format=json"])
            .args(extra)
            .env_remove("CODEGUARD_JOBS")
            .env_remove("CODEGUARD_TIMEOUT");
        if let Some(value) = timeout {
            command.env("CODEGUARD_TIMEOUT", value);
        }
        let output = command.output().expect("check all execution");
        assert_eq!(
            output.status.code(),
            Some(3),
            "未完成检查必须退出 3：{}",
            String::from_utf8_lossy(&output.stdout)
        );
        serde_json::from_slice(&output.stdout).expect("check report JSON")
    }

    /// 写入一个会在原生启动时留下标记的 Maven 替身，用于证明未启动原生探针。
    fn never_launch_tool(&self) -> PathBuf {
        let marker = self.0.join("native-launched");
        let tool = self.0.join("mvn");
        fs::write(&tool, format!("#!/bin/sh\ntouch '{}'\n", marker.display())).expect("tool");
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).expect("executable");
        tool
    }

    /// 提供完整原生先决条件：替身 Maven、JDK 目录与带摘要的隔离仓库。
    fn native_prerequisites(&self, script: &str) -> Vec<String> {
        let tool = self.0.join("mvn");
        fs::write(&tool, script).expect("tool");
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).expect("executable");
        let java_home = self.0.join("jdk");
        fs::create_dir_all(java_home.join("bin")).expect("jdk bin");
        fs::write(java_home.join("bin/java"), b"fixture java").expect("java");
        let repo = self.0.join("repo");
        fs::create_dir(&repo).expect("repo");
        fs::write(repo.join("artifact.jar"), b"fixture").expect("artifact");
        let digest = hash_bundle_tree(&repo).expect("repo digest");
        [
            "--maven-tool".to_string(),
            tool.to_string_lossy().into_owned(),
            "--java-home".to_string(),
            java_home.to_string_lossy().into_owned(),
            "--maven-repo".to_string(),
            repo.to_string_lossy().into_owned(),
            "--repo-sha256".to_string(),
            digest,
        ]
        .into()
    }

    fn report_java<'a>(&self, report: &'a Value) -> &'a Value {
        &report["native_results"]["java_p3c"]
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).expect("remove fixture");
    }
}

/// 禁用规则：P3C 插件显式 skip=true 属于 invalid 配置问题，不得启动原生探针，
/// 也不得把禁用报成「无问题」或「缺配置」。
#[test]
fn p3c_disabled_plugin_is_invalid_and_never_launches_native() {
    let project = Project::new();
    project.write_pom(&CONFIGURED_POM.replace(
        "        <configuration>\n",
        "        <configuration>\n          <skip>true</skip>\n",
    ));
    let tool = project.never_launch_tool();
    let report = project.check(&["--maven-tool", tool.to_str().unwrap()]);
    let java = project.report_java(&report);
    assert!(
        !project.0.join("native-launched").exists(),
        "invalid 配置不得启动原生 Maven"
    );
    let file = &java["files"][0];
    assert_eq!(file["configuration"], "invalid", "{java}");
    // 底层检查器原因必须随扫描输出，否则 invalid 与 missing/unknown 对用户不可区分。
    assert_eq!(
        file["configuration_reason"], "p3c_plugin_explicitly_skipped",
        "{java}"
    );
    assert_eq!(
        file["configuration_next_action"], "移除 Maven PMD 的 skip=true 并重新探测",
        "{java}"
    );
    assert_eq!(file["reason"], "p3c_configuration_not_confirmed");
    assert!(file["observation"].is_null());
    assert!(java["findings"].as_array().unwrap().is_empty());
    assert_eq!(java["observed_file_count"], 0);
    assert_eq!(report["delivery_decision"], "incomplete");
}

/// management-only：仅 pluginManagement 声明 P3C 不能生效，保持 unknown 并拒绝原生启动。
#[test]
fn p3c_management_only_declaration_is_unknown_and_never_launches_native() {
    let project = Project::new();
    project.write_pom(
        &CONFIGURED_POM
            .replace(
                "  <build>\n    <plugins>",
                "  <build>\n    <pluginManagement>\n      <plugins>",
            )
            .replace(
                "    </plugins>\n  </build>",
                "      </plugins>\n    </pluginManagement>\n  </build>",
            ),
    );
    let tool = project.never_launch_tool();
    let report = project.check(&["--maven-tool", tool.to_str().unwrap()]);
    let java = project.report_java(&report);
    assert!(!project.0.join("native-launched").exists());
    let file = &java["files"][0];
    assert_eq!(file["configuration"], "unknown", "{java}");
    assert_eq!(
        file["configuration_reason"], "effective_model_or_profile_not_resolved",
        "{java}"
    );
    assert_eq!(file["reason"], "p3c_configuration_not_confirmed");
    assert!(file["observation"].is_null());
    assert!(java["findings"].as_array().unwrap().is_empty());
}

/// profile 限定：P3C 只在 <profile> 内声明时不能视为项目已配置。
#[test]
fn p3c_profile_only_declaration_is_unknown_and_never_launches_native() {
    let project = Project::new();
    project.write_pom(
        &CONFIGURED_POM
            .replace(
                "  <build>\n    <plugins>",
                "  <profiles>\n    <profile>\n      <id>p3c</id>\n      <build>\n        <plugins>",
            )
            .replace(
                "    </plugins>\n  </build>",
                "        </plugins>\n      </build>\n    </profile>\n  </profiles>",
            ),
    );
    let tool = project.never_launch_tool();
    let report = project.check(&["--maven-tool", tool.to_str().unwrap()]);
    let java = project.report_java(&report);
    assert!(!project.0.join("native-launched").exists());
    let file = &java["files"][0];
    assert_eq!(file["configuration"], "unknown", "{java}");
    assert_eq!(
        file["configuration_reason"], "effective_model_or_profile_not_resolved",
        "{java}"
    );
    assert_eq!(file["reason"], "p3c_configuration_not_confirmed");
    assert!(file["observation"].is_null());
}

/// 配置存在不等于生效：直接插件完全 configured，但同一 POM 存在 <profiles> 段时
/// 规则集选择不可解析，必须保持未完成且不启动原生探针。
#[test]
fn configured_p3c_with_profile_section_keeps_ruleset_selection_unresolved() {
    let project = Project::new();
    project.write_pom(&CONFIGURED_POM.replace(
        "  <build>",
        "  <profiles>\n    <profile>\n      <id>ci</id>\n    </profile>\n  </profiles>\n  <build>",
    ));
    let tool = project.never_launch_tool();
    let report = project.check(&["--maven-tool", tool.to_str().unwrap()]);
    let java = project.report_java(&report);
    assert!(
        !project.0.join("native-launched").exists(),
        "规则集选择未解析时不得启动原生 Maven"
    );
    let file = &java["files"][0];
    assert_eq!(file["configuration"], "configured", "{java}");
    assert_eq!(
        file["configuration_reason"], "p3c_artifact_ruleset_and_error_policy_declared",
        "{java}"
    );
    assert_eq!(file["reason"], "p3c_ruleset_selection_unresolved");
    assert!(file["observation"].is_null());
    assert!(java["findings"].as_array().unwrap().is_empty());
    assert_eq!(java["observed_file_count"], 0);
    assert_eq!(report["delivery_decision"], "incomplete");
}

/// 未配置基线同样携带底层原因：missing 与 invalid/unknown 必须可区分。
#[test]
fn missing_p3c_declaration_keeps_specific_configuration_reason() {
    let project = Project::new();
    project.write_pom("<project><modelVersion>4.0.0</modelVersion></project>");
    let report = project.check(&[]);
    let java = project.report_java(&report);
    let file = &java["files"][0];
    assert_eq!(file["configuration"], "missing", "{java}");
    assert_eq!(
        file["configuration_reason"], "p3c_plugin_not_declared",
        "{java}"
    );
    assert_eq!(file["reason"], "p3c_configuration_not_confirmed");
    assert!(file["observation"].is_null());
}

/// 正反例相反：同一探针、同一规则集下，违规哨兵命中 ClassNamingShouldBeCamelRule，
/// 合法反例零诊断且不升格为质量通过（clean_scope_unproven）。
#[test]
fn clean_counterpart_opposes_the_violating_sentinel_under_one_probe() {
    let project = Project::new();
    let args = project.native_prerequisites(
        "#!/bin/sh\nmkdir -p target\nif [ -f src/main/java/Bad_Name.java ]; then\ncat > target/pmd.xml <<EOF\n<pmd xmlns=\"http://pmd.sourceforge.net/report/2.0.0\" version=\"6.15.0\"><file name=\"$PWD/src/main/java/Bad_Name.java\"><violation beginline=\"1\" endline=\"1\" begincolumn=\"7\" endcolumn=\"14\" rule=\"ClassNamingShouldBeCamelRule\" ruleset=\"AlibabaJavaNaming\" priority=\"2\">bad name</violation></file></pmd>\nEOF\nelse\nprintf '%s\\n' '<pmd xmlns=\"http://pmd.sourceforge.net/report/2.0.0\" version=\"6.15.0\"></pmd>' > target/pmd.xml\nfi\n",
    );
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let positive = project.check(&refs);
    let java = project.report_java(&positive);
    assert_eq!(java["finding_count"], 1, "{java}");
    assert_eq!(
        java["files"][0]["observation"]["findings"][0]["rule_id"], "ClassNamingShouldBeCamelRule",
        "{java}"
    );
    assert_eq!(java["local_observation_complete"], true);

    fs::remove_file(project.0.join("src/main/java/Bad_Name.java")).expect("remove sentinel");
    fs::write(
        project.0.join("src/main/java/GoodName.java"),
        "class GoodName {}\n",
    )
    .expect("legal counterpart");
    let negative = project.check(&refs);
    let java = project.report_java(&negative);
    assert_eq!(java["finding_count"], 0, "{java}");
    assert_eq!(java["observed_file_count"], 1, "{java}");
    assert_eq!(java["local_observation_complete"], true, "{java}");
    assert_eq!(
        java["files"][0]["observation"]["local_status"], "clean_scope_unproven",
        "{java}"
    );
    assert_eq!(
        java["files"][0]["observation"]["reason"], "clean_report_has_no_file_attestation",
        "{java}"
    );
    assert_eq!(java["coverage_proven"], false);
    assert_eq!(negative["delivery_decision"], "incomplete");
}

/// 坏 XML：原生返回不可解析报告时保持未完成，不得产生 findings 或清洁结论。
#[test]
fn malformed_native_report_keeps_scan_incomplete_without_findings() {
    let project = Project::new();
    let args = project.native_prerequisites(
        "#!/bin/sh\nmkdir -p target\nprintf '%s' '<pmd xmlns=\"http://pmd.sourceforge.net/report/2.0.0\" version=\"6.15.0\"><file nam' > target/pmd.xml\n",
    );
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let report = project.check(&refs);
    let java = project.report_java(&report);
    let file = &java["files"][0];
    assert_eq!(file["configuration"], "configured", "{java}");
    assert_eq!(file["reason"], "native_probe_returned", "{java}");
    assert_eq!(
        file["observation"]["reason"], "native_report_invalid_or_out_of_scope",
        "{java}"
    );
    assert_eq!(java["observed_file_count"], 0);
    assert!(java["findings"].as_array().unwrap().is_empty());
    assert_eq!(java["local_observation_complete"], false);
    assert_eq!(report["delivery_decision"], "incomplete");
}

/// 超时：整轮预算耗尽时原生执行被终止，文件保持未完成且不得借用旧报告。
#[test]
fn deadline_exhaustion_keeps_native_scan_incomplete() {
    let project = Project::new();
    let args = project.native_prerequisites(
        "#!/bin/sh\nsleep 30\nmkdir -p target\nprintf '%s\\n' '<pmd xmlns=\"http://pmd.sourceforge.net/report/2.0.0\" version=\"6.15.0\"></pmd>' > target/pmd.xml\n",
    );
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let report = project.check_env(&refs, Some("2s"));
    let java = project.report_java(&report);
    let file = &java["files"][0];
    assert!(
        file["observation"].is_null()
            || file["observation"]["findings"]
                .as_array()
                .is_some_and(Vec::is_empty),
        "超时后不得携带诊断：{java}"
    );
    assert_eq!(java["observed_file_count"], 0, "{java}");
    assert!(java["findings"].as_array().unwrap().is_empty());
    assert_eq!(java["local_observation_complete"], false, "{java}");
    assert_eq!(report["delivery_decision"], "incomplete");
}
