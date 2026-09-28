#![cfg(unix)]

use codeguard_runtime::observe_git_ancestry;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Repo(PathBuf);
impl Repo {
    fn new(format: &str) -> Self {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("cg-ancestry-{}-{id}", std::process::id()));
        fs::create_dir(&path).unwrap();
        let repo = Self(path);
        repo.git(&["init", "-q", &format!("--object-format={format}")]);
        repo
    }
    fn git(&self, args: &[&str]) -> String {
        let output = Command::new(git())
            .current_dir(&self.0)
            .args(args)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .output()
            .unwrap();
        assert!(output.status.success(), "{:?}", output.stderr);
        String::from_utf8(output.stdout).unwrap().trim().into()
    }
    fn commit(&self, parent: Option<&str>) -> String {
        let tree = self.git(&["mktree"]);
        let mut args = vec![
            "-c",
            "user.name=fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit-tree",
            &tree,
            "-m",
            "fixture",
        ];
        if let Some(parent) = parent {
            args.extend(["-p", parent]);
        }
        self.git(&args)
    }
    fn observe(&self, ancestor: &str, child: &str) -> Result<bool, &'static str> {
        observe_git_ancestry(
            &self.0,
            &git(),
            ancestor,
            child,
            Instant::now() + Duration::from_secs(10),
            &AtomicBool::new(false),
        )
    }
}
impl Drop for Repo {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn git() -> PathBuf {
    std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|dir| dir.join("git"))
        .find(|p| p.is_file())
        .unwrap()
        .canonicalize()
        .unwrap()
}

#[test]
fn native_complete_history_supports_sha1_sha256_and_rejects_reverse_or_unrelated() {
    for format in ["sha1", "sha256"] {
        let repo = Repo::new(format);
        let root = repo.commit(None);
        let child = repo.commit(Some(&root));
        let configuration = fs::read(repo.0.join(".git/config")).unwrap();
        fs::write(repo.0.join(".git/index"), b"unchanged-index-fixture").unwrap();
        let unrelated = repo.git(&[
            "-c",
            "user.name=fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit-tree",
            &repo.git(&["mktree"]),
            "-m",
            "different",
        ]);
        assert_eq!(repo.observe(&root, &child), Ok(true));
        assert_eq!(repo.observe(&child, &child), Ok(true));
        assert_eq!(repo.observe(&child, &root), Ok(false));
        assert_eq!(repo.observe(&unrelated, &child), Ok(false));
        let tree = repo.git(&["rev-parse", &format!("{child}^{{tree}}")]);
        assert!(repo.observe(&tree, &child).is_err());
        assert!(repo.observe(&"0".repeat(root.len()), &child).is_err());
        assert!(repo.observe("HEAD", &child).is_err());
        assert!(repo.observe(&"f".repeat(root.len()), &child).is_err());
        assert_eq!(fs::read(repo.0.join(".git/config")).unwrap(), configuration);
        assert_eq!(
            fs::read(repo.0.join(".git/index")).unwrap(),
            b"unchanged-index-fixture"
        );
    }
}

#[test]
fn replace_and_graft_cannot_invent_ancestry_and_shallow_history_is_incomplete() {
    let repo = Repo::new("sha1");
    let root = repo.commit(None);
    let other = repo.git(&[
        "-c",
        "user.name=fixture",
        "-c",
        "user.email=fixture@example.invalid",
        "commit-tree",
        &repo.git(&["mktree"]),
        "-m",
        "other",
    ]);
    let child = repo.commit(Some(&root));
    repo.git(&["replace", "--graft", &child, &other]);
    assert_eq!(repo.observe(&root, &child), Ok(true));
    assert_eq!(repo.observe(&other, &child), Ok(false));
    fs::write(
        repo.0.join(".git/info/grafts"),
        format!("{child} {other}\n"),
    )
    .unwrap();
    assert_eq!(repo.observe(&root, &child), Ok(true));
    assert_eq!(repo.observe(&other, &child), Ok(false));
    fs::write(repo.0.join(".git/shallow"), format!("{child}\n")).unwrap();
    assert_eq!(
        repo.observe(&root, &child),
        Err("git_ancestry_shallow_history")
    );
}

#[test]
fn absent_tool_cancelled_and_exhausted_budget_are_not_negative_or_positive_proofs() {
    let repo = Repo::new("sha1");
    let root = repo.commit(None);
    assert!(
        observe_git_ancestry(
            &repo.0,
            &git(),
            &root,
            &root,
            Instant::now(),
            &AtomicBool::new(false)
        )
        .is_err()
    );
    assert!(
        observe_git_ancestry(
            &repo.0,
            &git(),
            &root,
            &root,
            Instant::now() + Duration::from_secs(10),
            &AtomicBool::new(true)
        )
        .is_err()
    );
    assert!(
        observe_git_ancestry(
            &repo.0,
            &repo.0.join("missing-git"),
            &root,
            &root,
            Instant::now() + Duration::from_secs(10),
            &AtomicBool::new(false)
        )
        .is_err()
    );
}

#[test]
fn partial_clone_missing_object_does_not_start_a_promisor_transport() {
    use std::os::unix::fs::PermissionsExt;
    let repo = Repo::new("sha1");
    let root = repo.commit(None);
    let transport = repo.0.join("transport-fixture");
    fs::write(
        &transport,
        "#!/bin/sh\nprintf called > \"$0.invoked\"\nexit 1\n",
    )
    .unwrap();
    fs::set_permissions(&transport, fs::Permissions::from_mode(0o700)).unwrap();
    repo.git(&["config", "extensions.partialClone", "origin"]);
    repo.git(&["config", "remote.origin.promisor", "true"]);
    repo.git(&[
        "config",
        "remote.origin.url",
        &format!("ext::{}", transport.display()),
    ]);
    repo.git(&["config", "protocol.ext.allow", "always"]);
    assert!(repo.observe(&"f".repeat(40), &root).is_err());
    assert!(!repo.0.join("transport-fixture.invoked").exists());
    // 对照确实会调用本地假传输，避免把无效 partial-clone fixture 算作禁取对象证明。
    let control = Command::new(git())
        .current_dir(&repo.0)
        .args(["cat-file", "-t", &"f".repeat(40)])
        .env_clear()
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .unwrap();
    assert!(!control.status.success());
    assert!(
        repo.0.join("transport-fixture.invoked").exists(),
        "{:?}",
        control.stderr
    );
}
