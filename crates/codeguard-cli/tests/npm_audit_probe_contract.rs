#![cfg(unix)]
use codeguard_adapters::NpmAuditCommand;
use codeguard_cli::{
    npm_audit_probe::run_npm_audit_probe, npm_audit_probe_request::NpmAuditProbeRequest,
};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};
struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn exact_inputs_and_native_version_are_required_and_mutation_invalidates_observation() {
    use std::os::unix::fs::PermissionsExt;
    for mode in ["none", "lock", "npmrc"] {
        let changed = mode != "none";
        let root = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("cg-npm-probe-{}-{mode}", std::process::id()));
        fs::create_dir(&root).unwrap();
        let fixture = Fixture(root);
        fs::create_dir(fixture.0.join("evidence")).unwrap();
        fs::set_permissions(
            fixture.0.join("evidence"),
            fs::Permissions::from_mode(0o700),
        )
        .unwrap();
        let node = fixture.0.join("node");
        let mutation = match mode {
            "lock" => "printf changed >> package-lock.json",
            "npmrc" => "printf 'omit=dev' > .npmrc",
            _ => ":",
        };
        let script = format!(
            "#!/bin/sh\nif [ \"$3\" = --version ]; then printf '11.16.0\\n'; else {mutation}; printf '%s' '{{\"auditReportVersion\":2,\"vulnerabilities\":{{}},\"metadata\":{{\"vulnerabilities\":{{\"info\":0,\"low\":0,\"moderate\":0,\"high\":0,\"critical\":0,\"total\":0}},\"dependencies\":{{\"prod\":1,\"dev\":0,\"optional\":0,\"peer\":0,\"peerOptional\":0,\"total\":0}}}}}}'; fi\n"
        );
        fs::write(&node, script).unwrap();
        fs::set_permissions(&node, fs::Permissions::from_mode(0o700)).unwrap();
        for (file, bytes) in [
            ("npm.js", "entry"),
            ("package.json", "{}"),
            (
                "package-lock.json",
                r#"{"lockfileVersion":3,"packages":{"":{}}}"#,
            ),
            ("user.npmrc", ""),
            ("global.npmrc", ""),
        ] {
            fs::write(fixture.0.join(file), bytes).unwrap();
        }
        let inputs = [
            "node",
            "npm.js",
            "package.json",
            "package-lock.json",
            "user.npmrc",
            "global.npmrc",
        ];
        let mut req = NpmAuditProbeRequest {
            command: NpmAuditCommand {
                node,
                entry: fixture.0.join("npm.js"),
                cache: fixture.0.join("cache"),
                user_config: fixture.0.join("user.npmrc"),
                global_config: fixture.0.join("global.npmrc"),
            },
            cwd: fixture.0.clone(),
            evidence_dir: fixture.0.join("evidence"),
            run_id: "probe".into(),
            expected_version: "11.16.0".into(),
            registry: None,
            expected_sha256: inputs
                .into_iter()
                .map(|p| {
                    let path = fixture.0.join(p);
                    let hash = Sha256::digest(fs::read(&path).unwrap()).into();
                    (path, hash)
                })
                .collect::<BTreeMap<_, _>>(),
            deadline: Instant::now() + Duration::from_secs(10),
        };
        assert_eq!(
            run_npm_audit_probe(&req, &AtomicBool::new(true)).reason,
            Some("cancelled")
        );
        let deadline = req.deadline;
        req.deadline = Instant::now();
        assert_eq!(
            run_npm_audit_probe(&req, &AtomicBool::new(false)).reason,
            Some("deadline")
        );
        req.deadline = deadline;
        let missing = req.expected_sha256.remove(&req.command.entry).unwrap();
        assert_eq!(
            run_npm_audit_probe(&req, &AtomicBool::new(false)).reason,
            Some("npm_input_identity_missing")
        );
        req.expected_sha256
            .insert(req.command.entry.clone(), missing);
        req.expected_version = "11.16.1".into();
        req.run_id = "wrong-version".into();
        assert_eq!(
            run_npm_audit_probe(&req, &AtomicBool::new(false)).reason,
            Some("npm_version_mismatch")
        );
        req.expected_version = "11.16.0".into();
        req.run_id = "probe".into();
        let result = run_npm_audit_probe(&req, &AtomicBool::new(false));
        assert_eq!(
            result.local_coherent, !changed,
            "reason={:?}",
            result.reason
        );
        if changed {
            assert_eq!(result.reason, Some("npm_input_changed"));
        } else {
            assert_eq!(result.parsed.unwrap().advisory_coverage, "not_evaluated");
        }
    }
}
