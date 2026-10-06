#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use codeguard_cli::tool_identity::hash_bundle_tree;
use serde_json::Value;
use sha2::{Digest, Sha256};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Project(PathBuf);

impl Project {
    fn new() -> Self {
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "codeguard-cve-attribution-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::create_dir_all(root.join("src/main/java")).unwrap();
        fs::write(root.join("src/main/java/A.java"), b"class A {}\n").unwrap();
        fs::write(root.join("pom.xml"), b"<project><modelVersion>4.0.0</modelVersion><groupId>demo</groupId><artifactId>app</artifactId><version>1</version><build><plugins><plugin><groupId>org.apache.maven.plugins</groupId><artifactId>maven-dependency-plugin</artifactId><version>3.8.1</version></plugin><plugin><groupId>org.owasp</groupId><artifactId>dependency-check-maven</artifactId><version>12.1.0</version></plugin></plugins></build><dependencies><dependency><groupId>org.demo</groupId><artifactId>lib</artifactId><version>2.0</version></dependency></dependencies></project>").unwrap();
        fs::create_dir_all(root.join("repo/org/demo/lib/2.0")).unwrap();
        fs::write(
            root.join("repo/org/demo/lib/2.0/lib-2.0.jar"),
            b"artifact-bytes",
        )
        .unwrap();
        fs::create_dir(root.join("db")).unwrap();
        fs::write(root.join("db/odc.mv.db"), b"fixture-db").unwrap();
        fs::create_dir_all(root.join("jdk/bin")).unwrap();
        fs::write(root.join("jdk/bin/java"), b"fixture-java").unwrap();
        fs::write(root.join("jdk/release"), b"JAVA_VERSION=21\n").unwrap();
        Self(root)
    }

    fn check(&self, reported_digest: &str, private_package: bool) -> (i32, Value, String) {
        let script = self.0.join("mvn");
        let extra_package = if private_package {
            r#",{"id":"pkg:maven/org.demo/lib@2.0?repository_url=https://private.example/token"}"#
        } else {
            ""
        };
        let body = format!(
            r##"#!/bin/sh
for arg in "$@"; do
 case "$arg" in
  -DoutputFile=*) out=${{arg#-DoutputFile=}}; kind=tree ;;
  -Dodc.outputDirectory=*) out=${{arg#-Dodc.outputDirectory=}}; kind=cve ;;
 esac
done
if [ "$kind" = tree ]; then
cat > "$out" <<'EOF'
{{"groupId":"demo","artifactId":"app","version":"1","type":"jar","scope":"","classifier":"","optional":"false","children":[{{"groupId":"org.demo","artifactId":"lib","version":"2.0","type":"jar","scope":"compile","classifier":"","optional":"false","children":[]}}]}}
EOF
printf '[INFO] --- dependency:3.8.1:tree (default-cli) @ app ---\n[INFO] BUILD SUCCESS\n'
else
mkdir -p "$out"
cat > "$out/dependency-check-report.json" <<'EOF'
{{"reportSchema":"1.1","scanInfo":{{"engineVersion":"12.1.0","dataSource":[{{"name":"NVD","timestamp":"2026-09-25T00:00:00Z"}}]}},"projectInfo":{{"name":"app","reportDate":"2026-09-26T00:00:00Z"}},"dependencies":[{{"fileName":"lib.jar","isVirtual":false,"sha256":"{reported_digest}","packages":[{{"id":"pkg:maven/org.demo/lib@2.0"}}{extra_package}],"vulnerabilities":[{{"source":"NVD","name":"CVE-2026-1234","cvssv3":{{"baseScore":8.0}}}}]}}]}}
EOF
printf '[INFO] --- dependency-check:12.1.0:check (default-cli) @ app ---\n[INFO] BUILD SUCCESS\n'
fi
"##
        );
        fs::write(&script, body).unwrap();
        fs::set_permissions(&script, fs::Permissions::from_mode(0o700)).unwrap();
        let repo_sha = hash_bundle_tree(&self.0.join("repo")).unwrap();
        let db_sha = hash_bundle_tree(&self.0.join("db")).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["check", "java", self.0.to_str().unwrap(), "--format=json"])
            .args([
                "--maven-tool",
                script.to_str().unwrap(),
                "--java-home",
                self.0.join("jdk").to_str().unwrap(),
                "--maven-repo",
                self.0.join("repo").to_str().unwrap(),
                "--repo-sha256",
                &repo_sha,
                "--cve-data-dir",
                self.0.join("db").to_str().unwrap(),
                "--cve-data-sha256",
                &db_sha,
            ])
            .output()
            .unwrap();
        if let Some(dir) = std::env::var_os("CODEGUARD_JAVA_MIXED_BUILD_REPORT_DIR") {
            let value: Value = serde_json::from_slice(&output.stdout).unwrap();
            if value["schema_version"] == "0.61.0" {
                fs::create_dir_all(&dir).unwrap();
                fs::write(
                    PathBuf::from(dir).join(format!(
                        "native-attribution-{}-{}.json",
                        std::process::id(),
                        NEXT.fetch_add(1, Ordering::Relaxed)
                    )),
                    &output.stdout,
                )
                .unwrap();
            }
        }
        (
            output.status.code().unwrap(),
            serde_json::from_slice(&output.stdout).unwrap(),
            String::from_utf8(output.stderr).unwrap(),
        )
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn same_pom_native_reports_produce_exact_digest_candidate_only() {
    let project = Project::new();
    let digest = format!("{:x}", Sha256::digest(b"artifact-bytes"));
    let (exit, report, stderr) = project.check(&digest, false);
    assert_eq!(exit, 3, "{stderr}");
    assert_eq!(
        report["native_results"]["java_dependencies"]["observed_graph_count"], 1,
        "{report}"
    );
    assert_eq!(
        report["native_results"]["java_cve"]["observed_report_count"], 1,
        "{report}"
    );
    let candidate = &report["native_results"]["java_cve"]["attribution_probes"][0];
    assert_eq!(
        candidate["status"], "candidate_evaluated_untrusted",
        "{report}"
    );
    assert_eq!(
        candidate["bindings"][0]["coordinate_state"],
        "exact_coordinate_candidate"
    );
    assert_eq!(
        candidate["bindings"][0]["artifact_state"],
        "exact_digest_candidate"
    );
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert_eq!(
        report["native_results"]["java_cve"]["database_freshness"],
        "unverified"
    );

    let (exit, changed, _) = project.check(&"b".repeat(64), false);
    assert_eq!(exit, 3);
    assert_eq!(
        changed["native_results"]["java_cve"]["attribution_probes"][0]["bindings"][0]["artifact_state"],
        "digest_mismatch"
    );
    assert_eq!(changed["delivery_decision"], "not_evaluated");

    let (exit, private, _) = project.check(&digest, true);
    assert_eq!(exit, 3);
    assert_eq!(
        private["native_results"]["java_cve"]["attribution_probes"][0]["bindings"][0]["coordinate_state"],
        "unsupported_purl"
    );
    assert_eq!(
        private["native_results"]["java_cve"]["attribution_probes"][0]["bindings"][0]["artifact_state"],
        "attribution_unavailable"
    );
    assert!(!private.to_string().contains("private.example"));
}

#[test]
fn mixed_build_root_keeps_maven_native_evidence_without_assigning_it_to_gradle() {
    let project = Project::new();
    fs::write(project.0.join("build.gradle.kts"), b"plugins { java }\n").unwrap();
    let digest = format!("{:x}", Sha256::digest(b"artifact-bytes"));
    let (exit, report, stderr) = project.check(&digest, false);
    assert_eq!(exit, 3, "{stderr}");
    assert_eq!(
        report["native_results"]["java_cve"]["observed_report_count"], 1,
        "{report}"
    );
    assert_eq!(
        report["native_results"]["java_dependencies"]["observed_graph_count"], 1,
        "{report}"
    );
    assert_eq!(
        report["native_results"]["java_cve"]["probes"][0]["observation"]["advisories"][0]["advisory_id"],
        "CVE-2026-1234"
    );
    for category in ["cve", "dependencies", "security"] {
        let row = report["category_candidates"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["language"] == "java" && row["category"] == category)
            .unwrap();
        assert_eq!(row["checker_id"], Value::Null, "{report}");
        assert_eq!(row["status"], "configuration_unresolved", "{report}");
        assert_eq!(row["reason"], "checker_build_systems_mixed", "{report}");
    }
}
