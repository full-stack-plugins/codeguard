#![cfg(unix)]

use codeguard_cli::pmd6_probe::{Pmd6ProbeRequest, Pmd6ProbeState, run_pmd6_probe};
use codeguard_cli::tool_identity::hash_bundle_tree;
use sha2::{Digest, Sha256};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    root: PathBuf,
    source: PathBuf,
    launcher: PathBuf,
    bundle: PathBuf,
    evidence: PathBuf,
    reports: PathBuf,
}

impl Fixture {
    fn new(xml: &str, exit_code: i32) -> Self {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir()
            .canonicalize()
            .expect("physical temp root")
            .join(format!("codeguard-pmd6-probe-{}-{id}", std::process::id()));
        fs::create_dir(&root).expect("fixture root");
        let evidence = root.join("evidence");
        let reports = root.join("reports");
        let bundle = root.join("bundle");
        fs::create_dir(&bundle).expect("tool bundle");
        fs::write(bundle.join("pmd-core.jar"), b"pinned fixture jar").expect("delegated artifact");
        for directory in [&evidence, &reports] {
            fs::create_dir(directory).expect("private directory");
            fs::set_permissions(directory, fs::Permissions::from_mode(0o700))
                .expect("private mode");
        }
        let source = root.join("GoodName.java");
        fs::write(&source, b"class GoodName {}\n").expect("source");
        let launcher = bundle.join("run.sh");
        let shell = format!(
            "#!/bin/sh\nreport=\nwhile [ \"$#\" -gt 0 ]; do\n  if [ \"$1\" = -r ]; then shift; report=$1; fi\n  shift\ndone\nprintf '%s' '{}' > \"$report\"\nexit {exit_code}\n",
            xml
        );
        fs::write(&launcher, shell).expect("launcher");
        fs::set_permissions(&launcher, fs::Permissions::from_mode(0o700)).expect("executable");
        Self {
            root,
            source,
            launcher,
            bundle,
            evidence,
            reports,
        }
    }

    fn request(&self) -> Pmd6ProbeRequest {
        Pmd6ProbeRequest {
            source: self.source.clone(),
            launcher: self.launcher.clone(),
            expected_launcher_sha256: Sha256::digest(fs::read(&self.launcher).expect("launcher"))
                .into(),
            bundle_root: self.bundle.clone(),
            expected_bundle_sha256: hash_bundle_tree(&self.bundle).expect("fixture bundle"),
            ruleset: "category/java/bestpractices.xml".into(),
            report_dir: self.reports.clone(),
            evidence_dir: self.evidence.clone(),
            run_id: "probe".into(),
            expected_pmd_version: "6.15.0".into(),
            deadline: Instant::now() + Duration::from_secs(5),
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).expect("remove fixture");
    }
}

fn xml(source: &Path, violation: bool) -> String {
    let entry = if violation {
        "<violation beginline=\"1\" begincolumn=\"1\" endline=\"1\" endcolumn=\"5\" rule=\"Rule\" ruleset=\"Test\" priority=\"3\">issue</violation>"
    } else {
        ""
    };
    format!(
        "<pmd xmlns=\"http://pmd.sourceforge.net/report/2.0.0\" version=\"6.15.0\"><file name=\"{}\">{entry}</file></pmd>",
        source.display()
    )
}

#[test]
fn matching_native_violation_and_exit_is_local_evidence_only() {
    let fixture = Fixture::new("placeholder", 4);
    fs::write(&fixture.launcher, format!("#!/bin/sh\nreport=\nwhile [ \"$#\" -gt 0 ]; do\n if [ \"$1\" = -r ]; then shift; report=$1; fi\n shift\ndone\nprintf '%s' '{}' > \"$report\"\nexit 4\n", xml(&fixture.source, true))).expect("launcher");
    let result = run_pmd6_probe(&fixture.request(), &AtomicBool::new(false));
    assert_eq!(result.state, Pmd6ProbeState::LocalReportCoherent);
    assert_eq!(result.parsed.expect("parsed").diagnostics.len(), 1);
    assert!(fixture.reports.join("probe-pmd.xml").exists());
}

#[test]
fn contradictory_clean_exit_cannot_pass() {
    let fixture = Fixture::new("placeholder", 0);
    fs::write(&fixture.launcher, format!("#!/bin/sh\nreport=\nwhile [ \"$#\" -gt 0 ]; do\n if [ \"$1\" = -r ]; then shift; report=$1; fi\n shift\ndone\nprintf '%s' '{}' > \"$report\"\nexit 0\n", xml(&fixture.source, true))).expect("launcher");
    let result = run_pmd6_probe(&fixture.request(), &AtomicBool::new(false));
    assert_eq!(result.state, Pmd6ProbeState::Incomplete);
    assert_eq!(result.reason, Some("pmd_exit_report_conflict"));
}

#[test]
fn old_report_blocks_execution_and_is_preserved() {
    let fixture = Fixture::new("invalid", 0);
    fs::write(fixture.reports.join("probe-pmd.xml"), b"old").expect("old report");
    let result = run_pmd6_probe(&fixture.request(), &AtomicBool::new(false));
    assert_eq!(result.state, Pmd6ProbeState::Incomplete);
    assert_eq!(result.reason, Some("report_preflight_failed"));
    assert_eq!(
        fs::read(fixture.reports.join("probe-pmd.xml")).expect("old"),
        b"old"
    );
}

#[test]
fn wrong_source_in_report_is_incomplete() {
    let fixture = Fixture::new("placeholder", 4);
    fs::write(&fixture.launcher, format!("#!/bin/sh\nreport=\nwhile [ \"$#\" -gt 0 ]; do\n if [ \"$1\" = -r ]; then shift; report=$1; fi\n shift\ndone\nprintf '%s' '{}' > \"$report\"\nexit 4\n", xml(Path::new("/other/File.java"), true))).expect("launcher");
    let result = run_pmd6_probe(&fixture.request(), &AtomicBool::new(false));
    assert_eq!(result.state, Pmd6ProbeState::Incomplete);
    assert_eq!(result.reason, Some("report_source_mismatch"));
}

#[test]
fn matching_clean_report_and_exit_is_local_evidence_only() {
    let fixture = Fixture::new("placeholder", 0);
    fs::write(&fixture.launcher, format!("#!/bin/sh\nreport=\nwhile [ \"$#\" -gt 0 ]; do\n if [ \"$1\" = -r ]; then shift; report=$1; fi\n shift\ndone\nprintf '%s' '{}' > \"$report\"\nexit 0\n", xml(&fixture.source, false))).expect("launcher");
    let result = run_pmd6_probe(&fixture.request(), &AtomicBool::new(false));
    assert_eq!(result.state, Pmd6ProbeState::LocalReportCoherent);
    assert!(result.parsed.expect("parsed").diagnostics.is_empty());
}

#[test]
fn native_processing_exit_is_never_a_source_violation() {
    let fixture = Fixture::new("placeholder", 1);
    fs::write(&fixture.launcher, format!("#!/bin/sh\nreport=\nwhile [ \"$#\" -gt 0 ]; do\n if [ \"$1\" = -r ]; then shift; report=$1; fi\n shift\ndone\nprintf '%s' '{}' > \"$report\"\nexit 1\n", xml(&fixture.source, false))).expect("launcher");
    let result = run_pmd6_probe(&fixture.request(), &AtomicBool::new(false));
    assert_eq!(result.state, Pmd6ProbeState::Incomplete);
    assert_eq!(result.reason, Some("pmd_exit_report_conflict"));
    assert!(result.parsed.expect("parsed").diagnostics.is_empty());
}

#[test]
fn delegated_artifact_change_during_execution_is_incomplete() {
    let fixture = Fixture::new("placeholder", 0);
    fs::write(&fixture.launcher, format!("#!/bin/sh\nreport=\nwhile [ \"$#\" -gt 0 ]; do\n if [ \"$1\" = -r ]; then shift; report=$1; fi\n shift\ndone\nprintf '%s' '{}' > \"$report\"\nprintf tampered > '{}'\nexit 0\n", xml(&fixture.source, false), fixture.bundle.join("pmd-core.jar").display())).expect("launcher");
    let result = run_pmd6_probe(&fixture.request(), &AtomicBool::new(false));
    assert_eq!(result.state, Pmd6ProbeState::Incomplete);
    assert_eq!(result.reason, Some("bundle_identity_changed"));
}

#[test]
fn launcher_outside_locked_bundle_is_rejected() {
    let fixture = Fixture::new("placeholder", 0);
    let mut request = fixture.request();
    let outside = fixture.root.join("outside-run.sh");
    fs::copy(&fixture.launcher, &outside).expect("outside launcher");
    fs::set_permissions(&outside, fs::Permissions::from_mode(0o700)).expect("executable");
    request.launcher = outside;
    let result = run_pmd6_probe(&request, &AtomicBool::new(false));
    assert_eq!(result.state, Pmd6ProbeState::Incomplete);
    assert_eq!(result.reason, Some("launcher_outside_bundle"));
}
