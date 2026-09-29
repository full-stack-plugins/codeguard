#![cfg(unix)]

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use codeguard_cli::git_index_safety::{
    observe_index_safety, parse_index_listing, verify_git_blob_oid,
};
use serde_json::Value;
use sha2::Digest;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Repo(PathBuf);

impl Repo {
    fn new() -> Self {
        Self::new_with_format(None)
    }

    fn new_with_format(format: Option<&str>) -> Self {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("cg-index-{}-{id}", std::process::id()));
        fs::create_dir(&path).unwrap();
        let repo = Self(path);
        match format {
            Some("sha256") => repo.git(&["init", "-q", "--object-format=sha256"]),
            None => repo.git(&["init", "-q"]),
            _ => panic!("unsupported test format"),
        }
        repo
    }

    fn git(&self, args: &[&str]) {
        assert!(
            Command::new(git_binary())
                .current_dir(&self.0)
                .args(args)
                .status()
                .unwrap()
                .success()
        );
    }
}

impl Drop for Repo {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn git_binary() -> PathBuf {
    std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|dir| dir.join("git"))
        .find(|path| path.is_file())
        .unwrap()
        .canonicalize()
        .unwrap()
}

fn synthetic_ed25519_pem() -> String {
    fn field(out: &mut Vec<u8>, value: &[u8]) {
        out.extend_from_slice(&(value.len() as u32).to_be_bytes());
        out.extend_from_slice(value);
    }
    let public_key = [0xa5_u8; 32];
    let mut public = Vec::new();
    field(&mut public, b"ssh-ed25519");
    field(&mut public, &public_key);
    let mut private = Vec::new();
    private.extend_from_slice(&0x1234_5678_u32.to_be_bytes());
    private.extend_from_slice(&0x1234_5678_u32.to_be_bytes());
    field(&mut private, b"ssh-ed25519");
    field(&mut private, &public_key);
    let mut key_material = [0x5a_u8; 64];
    key_material[32..].copy_from_slice(&public_key);
    field(&mut private, &key_material);
    field(&mut private, b"");
    private.extend((1..=(8 - private.len() % 8)).map(|value| value as u8));
    let mut envelope = b"openssh-key-v1\0".to_vec();
    field(&mut envelope, b"none");
    field(&mut envelope, b"none");
    field(&mut envelope, b"");
    envelope.extend_from_slice(&1_u32.to_be_bytes());
    field(&mut envelope, &public);
    field(&mut envelope, &private);
    format!(
        "-----BEGIN OPENSSH PRIVATE KEY-----\n{}\n-----END OPENSSH PRIVATE KEY-----\n",
        STANDARD.encode(envelope)
    )
}

#[test]
fn staged_private_key_material_is_found_in_managed_records_without_reading_worktree_bytes() {
    let repo = Repo::new();
    fs::create_dir_all(repo.0.join(".codeguard/findings")).unwrap();
    let record = repo.0.join(".codeguard/findings/CG-demo.md");
    fs::write(&record, synthetic_ed25519_pem()).unwrap();
    repo.git(&["add", ".codeguard/findings/CG-demo.md"]);
    fs::write(&record, "safe current worktree text\n").unwrap();
    let observed = observe_index_safety(&repo.0, &git_binary(), None).unwrap();
    assert!(observed.objects_verified);
    assert!(observed.violations.iter().any(|item| {
        item.path == ".codeguard/findings/CG-demo.md"
            && item.rule_id == "repository_policy.unencrypted_openssh_ed25519_private_key"
    }));
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "gate",
            "pre-commit",
            repo.0.to_str().unwrap(),
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["schema_version"], "0.3.0");
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert_eq!(
        report["violations"][0]["rule_id"],
        "repository_policy.unencrypted_openssh_ed25519_private_key"
    );
    assert!(!String::from_utf8_lossy(&output.stdout).contains("BEGIN OPENSSH"));
}

#[test]
fn unstaged_private_key_material_does_not_change_the_staged_content_result() {
    let repo = Repo::new();
    fs::write(repo.0.join("notes.md"), "safe staged bytes\n").unwrap();
    repo.git(&["add", "notes.md"]);
    fs::write(repo.0.join("notes.md"), synthetic_ed25519_pem()).unwrap();
    let observed = observe_index_safety(&repo.0, &git_binary(), None).unwrap();
    assert!(observed.objects_verified);
    assert!(observed.violations.is_empty());
}

#[test]
fn staged_hidden_secret_is_seen_but_unstaged_worktree_secret_is_not() {
    let repo = Repo::new();
    fs::create_dir_all(repo.0.join(".github/workflows")).unwrap();
    fs::write(repo.0.join(".github/workflows/ci.yml"), "name: CI\n").unwrap();
    fs::write(repo.0.join(".env"), "TOKEN=fixture\n").unwrap();
    fs::write(repo.0.join("untracked.pem"), "not staged\n").unwrap();
    repo.git(&["add", "-f", ".github/workflows/ci.yml", ".env"]);
    let actual = observe_index_safety(&repo.0, &git_binary(), None).unwrap();
    assert_eq!(actual.entries.len(), 2);
    assert_eq!(actual.violations.len(), 1);
    assert_eq!(actual.violations[0].path, ".env");
    assert!(
        !actual
            .entries
            .iter()
            .any(|entry| entry.path == "untracked.pem")
    );
}

#[test]
fn alternate_index_is_observed_without_modifying_default_index() {
    let repo = Repo::new();
    fs::write(repo.0.join("ordinary.py"), "pass\n").unwrap();
    repo.git(&["add", "ordinary.py"]);
    fs::write(repo.0.join(".env"), "TOKEN=fixture\n").unwrap();
    let alternate = repo.0.join("alternate.index");
    let output = Command::new(git_binary())
        .current_dir(&repo.0)
        .env("GIT_INDEX_FILE", &alternate)
        .args(["add", "-f", ".env"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let normal = observe_index_safety(&repo.0, &git_binary(), None).unwrap();
    let staged = observe_index_safety(&repo.0, &git_binary(), Some(&alternate)).unwrap();
    assert!(normal.violations.is_empty());
    assert_eq!(staged.violations[0].path, ".env");
    assert_ne!(normal.listing_sha256, staged.listing_sha256);
    let after = observe_index_safety(&repo.0, &git_binary(), None).unwrap();
    assert_eq!(normal.listing_sha256, after.listing_sha256);
}

#[test]
fn malformed_or_conflicted_index_records_are_not_accepted() {
    assert!(parse_index_listing(b"100644 deadbeef 0\t.env\0", "sha1").is_err());
    let unmerged = format!("100644 {} 2\t.env\0", "a".repeat(40));
    assert!(parse_index_listing(unmerged.as_bytes(), "sha1").is_err());
    let non_utf8 = [
        b'1', b'0', b'0', b'6', b'4', b'4', b' ', b'0', b'\t', 0xff, 0,
    ];
    assert!(parse_index_listing(&non_utf8, "sha1").is_err());
}

#[test]
fn non_repository_or_missing_alternate_index_is_incomplete() {
    let directory = tempfile_path();
    fs::create_dir(&directory).unwrap();
    assert!(observe_index_safety(&directory, &git_binary(), None).is_err());
    fs::remove_dir_all(&directory).unwrap();
    let repo = Repo::new();
    assert!(
        observe_index_safety(&repo.0, &git_binary(), Some(&repo.0.join("missing.index"))).is_err()
    );
}

#[test]
fn real_sha256_repository_index_is_parsed_without_sha1_assumptions() {
    let repo = Repo::new_with_format(Some("sha256"));
    fs::write(repo.0.join(".env"), "TOKEN=fixture\n").unwrap();
    repo.git(&["add", "-f", ".env"]);
    let observed = observe_index_safety(&repo.0, &git_binary(), None).unwrap();
    assert_eq!(observed.object_format, "sha256");
    assert_eq!(observed.entries[0].oid.len(), 64);
    assert_eq!(observed.violations[0].path, ".env");
    assert!(observed.objects_verified);
}

#[test]
fn git_blob_hash_is_checked_independently_for_both_object_formats() {
    assert!(verify_git_blob_oid(
        b"hello\n",
        "sha1",
        "ce013625030ba8dba906f756967f9e9ca394464a"
    ));
    assert!(!verify_git_blob_oid(
        b"hello!\n",
        "sha1",
        "ce013625030ba8dba906f756967f9e9ca394464a"
    ));
    let repo = Repo::new_with_format(Some("sha256"));
    fs::write(repo.0.join("hello.txt"), "hello\n").unwrap();
    repo.git(&["add", "hello.txt"]);
    let observed = observe_index_safety(&repo.0, &git_binary(), None).unwrap();
    assert!(verify_git_blob_oid(
        b"hello\n",
        "sha256",
        &observed.entries[0].oid
    ));
    assert!(!verify_git_blob_oid(
        b"hello!\n",
        "sha256",
        &observed.entries[0].oid
    ));
}

#[test]
fn staged_bytes_are_verified_even_when_worktree_has_changed() {
    let repo = Repo::new();
    fs::write(repo.0.join("app.py"), "staged bytes\n").unwrap();
    repo.git(&["add", "app.py"]);
    fs::write(repo.0.join("app.py"), "different working tree bytes\n").unwrap();
    let observed = observe_index_safety(&repo.0, &git_binary(), None).unwrap();
    assert!(observed.objects_verified);
    assert_eq!(observed.object_evidence[0].path, "app.py");
    assert_eq!(
        observed.object_evidence[0].content_sha256,
        format!("{:x}", sha2::Sha256::digest(b"staged bytes\n"))
    );
}

#[test]
fn symlink_and_lfs_pointer_are_explicitly_unresolved_for_source_coverage() {
    use std::os::unix::fs::symlink;
    let repo = Repo::new();
    symlink("../outside", repo.0.join("link")).unwrap();
    fs::write(
        repo.0.join("asset.bin"),
        "version https://git-lfs.github.com/spec/v1\noid sha256:abc\nsize 10\n",
    )
    .unwrap();
    repo.git(&["add", "link", "asset.bin"]);
    let observed = observe_index_safety(&repo.0, &git_binary(), None).unwrap();
    assert!(!observed.objects_verified);
    assert_eq!(observed.unresolved_object_paths, ["asset.bin", "link"]);
    assert_eq!(observed.object_evidence.len(), 2);
}

#[test]
fn object_size_limit_does_not_hide_a_staged_path_violation() {
    let repo = Repo::new();
    fs::write(repo.0.join(".env"), vec![b'x'; 8 * 1024 * 1024 + 1]).unwrap();
    repo.git(&["add", "-f", ".env"]);
    let observed = observe_index_safety(&repo.0, &git_binary(), None).unwrap();
    assert_eq!(observed.violations[0].path, ".env");
    assert!(!observed.objects_verified);
    assert_eq!(observed.unresolved_object_paths, [".env"]);
    assert!(observed.object_verification_reason.is_some());
}

#[test]
fn oversized_staged_blob_does_not_erase_a_verified_private_key_finding() {
    let repo = Repo::new();
    fs::create_dir_all(repo.0.join(".codeguard/findings")).unwrap();
    fs::write(
        repo.0.join(".codeguard/findings/CG-demo.md"),
        synthetic_ed25519_pem(),
    )
    .unwrap();
    fs::write(repo.0.join("large.bin"), vec![b'x'; 8 * 1024 * 1024 + 1]).unwrap();
    repo.git(&["add", ".codeguard/findings/CG-demo.md", "large.bin"]);
    let observed = observe_index_safety(&repo.0, &git_binary(), None).unwrap();
    assert!(!observed.objects_verified);
    assert_eq!(observed.unresolved_object_paths, ["large.bin"]);
    assert_eq!(observed.object_evidence.len(), 1);
    assert!(observed.object_verification_reason.is_some());
    assert!(observed.violations.iter().any(|item| {
        item.path == ".codeguard/findings/CG-demo.md"
            && item.rule_id == "repository_policy.unencrypted_openssh_ed25519_private_key"
    }));
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "gate",
            "pre-commit",
            repo.0.to_str().unwrap(),
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["object_status"], "failed");
    assert_eq!(report["verified_object_count"], 1);
    assert_eq!(report["unresolved_object_count"], 1);
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert!(report["violations"].as_array().unwrap().iter().any(|item| {
        item["rule_id"] == "repository_policy.unencrypted_openssh_ed25519_private_key"
    }));
    assert!(!String::from_utf8_lossy(&output.stdout).contains("BEGIN OPENSSH"));
}

#[test]
fn public_gate_preview_reports_staged_violation_without_a_false_allow() {
    let repo = Repo::new();
    fs::write(repo.0.join(".env"), "TOKEN=fixture\n").unwrap();
    repo.git(&["add", "-f", ".env"]);
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "gate",
            "pre-commit",
            repo.0.to_str().unwrap(),
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    assert!(output.stderr.is_empty());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["index_observation"], "complete");
    assert_eq!(report["object_status"], "verified");
    assert_eq!(report["verified_object_count"], 1);
    assert_eq!(report["unresolved_object_count"], 0);
    assert_eq!(report["violations"][0]["path"], ".env");
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert_eq!(report["command_status"], "incomplete");
}

#[test]
fn public_gate_preview_rejects_bad_arguments_before_git_execution() {
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["gate", "pre-commit", "--format=yaml"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
}

#[test]
fn public_gate_preview_respects_git_index_file() {
    let repo = Repo::new();
    fs::write(repo.0.join("safe.py"), "pass\n").unwrap();
    repo.git(&["add", "safe.py"]);
    fs::write(repo.0.join(".env"), "TOKEN=fixture\n").unwrap();
    let alternate = repo.0.join("alternate.index");
    let staged = Command::new(git_binary())
        .current_dir(&repo.0)
        .env("GIT_INDEX_FILE", &alternate)
        .args(["add", "-f", ".env"])
        .output()
        .unwrap();
    assert!(staged.status.success());
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .env("GIT_INDEX_FILE", &alternate)
        .args([
            "gate",
            "pre-commit",
            repo.0.to_str().unwrap(),
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["violations"][0]["path"], ".env");
    let normal = observe_index_safety(&repo.0, &git_binary(), None).unwrap();
    assert!(normal.violations.is_empty());
}

#[test]
fn published_preview_schema_never_contains_an_allow_decision() {
    let schema: Value = serde_json::from_str(include_str!(
        "../../../schemas/git-index-safety-preview.schema.json"
    ))
    .unwrap();
    let previous: Value = serde_json::from_str(include_str!(
        "../../../schemas/git-index-safety-preview-0.2.schema.json"
    ))
    .unwrap();
    assert_eq!(
        schema["$id"],
        "urn:codeguard:schema:git-index-safety-preview:0.3.0"
    );
    assert_eq!(previous["properties"]["schema_version"]["const"], "0.2.0");
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(
        schema["properties"]["delivery_decision"]["const"],
        "not_evaluated"
    );
    assert_eq!(
        schema["properties"]["violations"]["items"]["additionalProperties"],
        false
    );
}

fn tempfile_path() -> PathBuf {
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("cg-not-repo-{}-{id}", std::process::id()))
}
