#![cfg(unix)]
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::{Duration, Instant},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Project(PathBuf);
impl Project {
    fn new() -> Self {
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-rust-edition-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("member/src")).unwrap();
        fs::write(root.join("member/src/lib.rs"), "pub async fn f() {}\n").unwrap();
        Self(root)
    }
    fn observe(&self, tool: &std::path::Path) -> serde_json::Value {
        codeguard_cli::rust_project_syntax::observe(
            &self.0,
            "member/src/lib.rs",
            tool,
            Instant::now() + Duration::from_secs(15),
            &AtomicBool::new(false),
        )
    }
}
impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn standalone_and_unresolved_workspace_do_not_start_a_parser() {
    let p = Project::new();
    assert_eq!(
        p.observe(std::path::Path::new("/missing/rustfmt"))["reason"],
        "rust_package_manifest_not_found"
    );
    fs::write(
        p.0.join("member/Cargo.toml"),
        "[package]\nname='m'\nedition.workspace=true\n",
    )
    .unwrap();
    assert_eq!(
        p.observe(std::path::Path::new("/missing/rustfmt"))["reason"],
        "rust_workspace_edition_unresolved"
    );
}

#[test]
fn edition_context_resolves_nearest_package_and_explicit_workspace_locator() {
    use codeguard_cli::rust_project_edition::RustProjectEdition;
    let p = Project::new();
    fs::write(
        p.0.join("Cargo.toml"),
        "[workspace.package]\nedition='2024'\n",
    )
    .unwrap();
    fs::write(
        p.0.join("member/Cargo.toml"),
        "[package]\nname='m'\nedition='2018'\n",
    )
    .unwrap();
    let direct = RustProjectEdition::capture(&p.0, "member/src/lib.rs").unwrap();
    assert_eq!(direct.edition, "2018");
    assert!(direct.current());
    fs::write(
        p.0.join("member/Cargo.toml"),
        "[package]\nname='m'\nedition.workspace=true\nworkspace='..'\n",
    )
    .unwrap();
    assert!(!direct.current());
    let inherited = RustProjectEdition::capture(&p.0, "member/src/lib.rs").unwrap();
    assert_eq!(inherited.edition, "2024");
    assert!(inherited.current());
    fs::write(
        p.0.join("Cargo.toml"),
        "[workspace.package]\nedition='2021'\n",
    )
    .unwrap();
    assert!(!inherited.current());
}

#[test]
fn default_edition_does_not_borrow_from_outer_package_or_follow_symlinks() {
    use codeguard_cli::rust_project_edition::RustProjectEdition;
    let p = Project::new();
    fs::write(
        p.0.join("Cargo.toml"),
        "[package]\nname='outer'\nedition='2024'\n",
    )
    .unwrap();
    fs::write(p.0.join("member/Cargo.toml"), "[package]\nname='m'\n").unwrap();
    assert_eq!(
        RustProjectEdition::capture(&p.0, "member/src/lib.rs")
            .unwrap()
            .edition,
        "2015"
    );
    fs::remove_file(p.0.join("member/Cargo.toml")).unwrap();
    std::os::unix::fs::symlink(p.0.join("Cargo.toml"), p.0.join("member/Cargo.toml")).unwrap();
    assert!(RustProjectEdition::capture(&p.0, "member/src/lib.rs").is_err());
}

#[test]
#[ignore = "requires explicit installed rustfmt1.9.0-stable"]
fn real_parser_uses_project_edition_instead_of_fixed_2024() {
    let p = Project::new();
    let tool = PathBuf::from(std::env::var("CODEGUARD_RUSTFMT_BIN").unwrap());
    let mut reports = Vec::new();
    for (declaration, edition, status) in [
        ("", "2015", "diagnostics_observed"),
        ("edition='2021'", "2021", "completed"),
        ("edition='2024'", "2024", "completed"),
    ] {
        fs::write(
            p.0.join("member/Cargo.toml"),
            format!("[package]\nname='m'\n{declaration}\n"),
        )
        .unwrap();
        let r = p.observe(&tool);
        assert_eq!(r["edition_context"]["edition"], edition, "{r}");
        assert_eq!(r["native"]["status"], status, "{r}");
        assert_eq!(r["delivery_decision"], "not_evaluated");
        reports.push(r);
    }
    if let Ok(path) = std::env::var("CODEGUARD_RUST_PROJECT_REPORT") {
        fs::write(path, serde_json::to_vec_pretty(&reports).unwrap()).unwrap();
    }
}

#[test]
fn a_new_nearer_manifest_invalidates_the_original_declaration_lookup() {
    let p = Project::new();
    fs::write(
        p.0.join("member/Cargo.toml"),
        "[package]\nname='m'\nedition='2024'\n",
    )
    .unwrap();
    let context =
        codeguard_cli::rust_project_edition::RustProjectEdition::capture(&p.0, "member/src/lib.rs")
            .unwrap();
    fs::write(
        p.0.join("member/src/Cargo.toml"),
        "[package]\nname='nested'\nedition='2015'\n",
    )
    .unwrap();
    assert!(!context.current());
}

#[test]
fn unresolved_nearest_workspace_and_outside_locator_cannot_borrow_outer_defaults() {
    let p = Project::new();
    fs::write(
        p.0.join("Cargo.toml"),
        "[workspace.package]\nedition='2024'\n",
    )
    .unwrap();
    fs::write(
        p.0.join("member/Cargo.toml"),
        "[package]\nname='m'\nedition.workspace=true\n[workspace]\n",
    )
    .unwrap();
    assert!(
        codeguard_cli::rust_project_edition::RustProjectEdition::capture(&p.0, "member/src/lib.rs")
            .is_err()
    );
    fs::write(
        p.0.join("member/Cargo.toml"),
        "[package]\nname='m'\nedition.workspace=true\nworkspace='../..'\n",
    )
    .unwrap();
    assert_eq!(
        p.observe(std::path::Path::new("/missing/rustfmt"))["reason"],
        "rust_workspace_locator_outside_root"
    );
}

#[test]
fn expired_budget_does_not_read_project_declarations_or_start_native() {
    let p = Project::new();
    let report = codeguard_cli::rust_project_syntax::observe(
        &p.0,
        "member/src/lib.rs",
        std::path::Path::new("/missing/rustfmt"),
        Instant::now(),
        &AtomicBool::new(false),
    );
    assert_eq!(report["reason"], "rust_project_syntax_not_started");
    assert!(report["edition_context"].is_null());
    assert!(report["native"].is_null());
}

#[test]
fn version_phase_manifest_change_prevents_the_second_native_invocation() {
    use std::os::unix::fs::PermissionsExt;
    let p = Project::new();
    let manifest = p.0.join("member/Cargo.toml");
    fs::write(&manifest, "[package]\nname='m'\nedition='2024'\n").unwrap();
    let tool = p.0.join("rustfmt");
    let marker = p.0.join("parser-started");
    fs::write(&tool,format!("#!/bin/sh\nif [ \"$1\" = --version ]; then printf \"[package]\\nname='m'\\nedition='2015'\\n\" > '{}'; printf 'rustfmt 1.9.0-stable (fixture)\\n'; else /usr/bin/touch '{}'; printf 'formatted\\n'; fi\n",manifest.display(),marker.display())).unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let report = p.observe(&tool);
    assert_eq!(
        report["reason"], "rust_project_syntax_inputs_changed",
        "{report}"
    );
    assert!(report["native"].is_null());
    assert!(!marker.exists());
}
