use codeguard_cli::distribution_source::{DistributionTrustKey, DistributionVerificationContext};
use codeguard_cli::tool_identity::current_platform_id;
use ring::signature::{Ed25519KeyPair, KeyPair};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};
pub fn sha(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
pub fn context() -> DistributionVerificationContext<'static> {
    DistributionVerificationContext {
        publisher_id: "publisher",
        channel: "stable",
        platform: current_platform_id().unwrap(),
        now_unix: Some(200),
        minimum_sequence: 1,
        max_lifetime_seconds: 400,
    }
}
pub fn trust() -> DistributionTrustKey {
    let pair = Ed25519KeyPair::from_seed_unchecked(&[23; 32]).unwrap();
    DistributionTrustKey {
        key_id: "key".into(),
        publisher_id: "publisher".into(),
        public_key: pair.public_key().as_ref().try_into().unwrap(),
        valid_from: 100,
        valid_until: 1000,
        revoked: false,
    }
}
pub fn inputs(package: &[u8], archive: bool) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    inputs_at_url(package, archive, "https://example.org/tool")
}
pub fn inputs_at_url(package: &[u8], archive: bool, url: &str) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let platform = current_platform_id().unwrap();
    let binary = sha(if archive { b"tool" } else { package });
    let (format, entry, tree, root) = if archive {
        let tree = codeguard_runtime::unpack_package_archive_tree(
            package,
            &codeguard_runtime::ArchiveUnpackRequest {
                format: "zip",
                entrypoint: "release/bin/tool",
                package_sha256: Sha256::digest(package).into(),
                entrypoint_sha256: Sha256::digest(b"tool").into(),
                max_unpacked_bytes: 1024,
            },
            Instant::now() + Duration::from_secs(5),
            &AtomicBool::new(false),
        )
        .unwrap();
        let h = codeguard_runtime::hash_unpacked_bundle_tree(
            &tree,
            Instant::now() + Duration::from_secs(5),
            &AtomicBool::new(false),
        )
        .unwrap();
        (
            "zip",
            Some("release/bin/tool"),
            Some(h.clone()),
            format!("{h}.bundle/release/bin/tool"),
        )
    } else {
        ("raw", None, None, format!("{binary}.bin"))
    };
    let lock=serde_json::to_vec(&json!({"schema_version":"1.0","lock_id":"test","tools":[{"id":"tool","version":"1","platform":platform,"binary_sha256":binary,"adapter":{"id":"tool","version":"1"},"rule_source":{"kind":"native_builtin","id":"tool","sha256":"b".repeat(64)},"origin":{"kind":"managed_cache","ref":root}}]})).unwrap();
    let manifest=serde_json::to_vec(&json!({"schema_version":"1.2","manifest_id":"test","lock_sha256":sha(&lock),"artifacts":[{"tool_id":"tool","version":"1","platform":platform,"binary_sha256":binary,"origin_ref_sha256":sha(root.as_bytes()),"download_url":url,"package_sha256":sha(package),"package_size_bytes":package.len(),"format":format,"entrypoint":entry,"unpacked_size_limit_bytes":if archive{1024}else{package.len()},"bundle_tree_sha256":null,"bundle_archive_root":null,"install_tree_sha256":tree}]})).unwrap();
    let payload=serde_json::to_string(&json!({"schema_version":"1.0","publisher_id":"publisher","channel":"stable","platform":platform,"release_sequence":1,"issued_at":150,"expires_at":500,"manifest_sha256":sha(&manifest),"lock_sha256":sha(&lock)})).unwrap();
    let mut message = b"codeguard.distribution.v1\0key\0".to_vec();
    message.extend_from_slice(payload.as_bytes());
    let pair = Ed25519KeyPair::from_seed_unchecked(&[23; 32]).unwrap();
    let signature = pair
        .sign(&message)
        .as_ref()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let signed=serde_json::to_vec(&json!({"schema_version":"1.0","key_id":"key","release_json":payload,"signature_hex":signature})).unwrap();
    (signed, manifest, lock)
}
pub fn cache() -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let p = std::env::temp_dir().join(format!(
        "codeguard-signed-publish-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&p).unwrap();
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o700)).unwrap();
    p
}
pub fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(5)
}
