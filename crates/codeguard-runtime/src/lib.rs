//! Codeguard 的受控本地文件观察基础设施。

use std::fs;
use std::io;
use std::io::Read;
use std::path::Path;

#[cfg(feature = "wasm-precheck")]
mod wasm_grammar;
#[cfg(feature = "wasm-precheck")]
pub use codeguard_core::SyntaxRecoveryAnchor as WasmRecovery;
#[cfg(feature = "wasm-precheck")]
pub use wasm_grammar::WasmGrammar;
#[cfg(feature = "wasm-precheck")]
mod wasm_recovery_scan;
#[cfg(feature = "wasm-precheck")]
pub use wasm_recovery_scan::{WasmRecoveryScan, scan_wasm_recoveries};

#[cfg(feature = "wasm-precheck")]
mod wasm_empty_block;
#[cfg(feature = "wasm-precheck")]
pub use wasm_empty_block::WasmEmptyBlock;
#[cfg(feature = "wasm-precheck")]
mod wasm_empty_block_scan;
#[cfg(feature = "wasm-precheck")]
pub use wasm_empty_block_scan::{WasmEmptyBlockScan, scan_wasm_empty_blocks};

#[cfg(feature = "wasm-precheck")]
mod wasm_root_child_scan;
#[cfg(feature = "wasm-precheck")]
pub use wasm_root_child_scan::{WasmRootChildScan, scan_wasm_root_child};

#[cfg(feature = "wasm-precheck")]
mod wasm_duplicate_binding;
#[cfg(feature = "wasm-precheck")]
pub use wasm_duplicate_binding::WasmDuplicateBinding;
#[cfg(feature = "wasm-precheck")]
mod wasm_sibling_binding_scan;
#[cfg(feature = "wasm-precheck")]
pub use wasm_sibling_binding_scan::{WasmSiblingBindingScan, scan_wasm_sibling_bindings};

#[cfg(unix)]
mod installed_artifact;
#[cfg(unix)]
pub use installed_artifact::InstalledArtifact;
#[cfg(unix)]
mod install_stage;
#[cfg(unix)]
mod tool_cache_install;
#[cfg(unix)]
pub use tool_cache_install::publish_tool_bytes;

#[cfg(any(target_os = "macos", target_os = "linux"))]
mod bundle_cache_fs;
#[cfg(any(target_os = "macos", target_os = "linux"))]
mod bundle_install_stage;
#[cfg(any(target_os = "macos", target_os = "linux"))]
mod installed_bundle;
#[cfg(any(target_os = "macos", target_os = "linux"))]
pub use installed_bundle::InstalledBundle;
#[cfg(any(target_os = "macos", target_os = "linux"))]
mod tool_bundle_install;
#[cfg(any(target_os = "macos", target_os = "linux"))]
pub use tool_bundle_install::publish_tool_bundle;

mod archive_unpack_request;
pub use archive_unpack_request::ArchiveUnpackRequest;
mod projected_bundle;
pub use projected_bundle::ProjectedBundle;
mod bundle_projection;
pub use bundle_projection::project_unpacked_bundle;
mod unpacked_archive;
pub use unpacked_archive::UnpackedArchive;
mod unpacked_bundle_tree;
pub use unpacked_bundle_tree::{hash_unpacked_bundle_tree, verify_unpacked_bundle_tree};
mod unpacked_file;
pub use unpacked_file::UnpackedFile;
mod package_archive;
mod tar_extension_state;
pub use package_archive::{unpack_package_archive, unpack_package_archive_tree};

mod package_bytes;
pub use package_bytes::read_verified_package;

mod approval_signature;
pub use approval_signature::verify_approval_signature;
#[cfg(unix)]
mod git_ancestry;
#[cfg(unix)]
pub use git_ancestry::observe_git_ancestry;

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

#[cfg(unix)]
mod interrupt;
mod native_observation;
#[cfg(unix)]
mod private_log;
mod process_runner;
mod process_spec;
mod source_snapshot;
#[cfg(unix)]
mod task_file_lock;
mod task_scheduler;
mod termination;

#[cfg(unix)]
pub use interrupt::install_sigint_cancellation;
pub use native_observation::NativeObservation;
#[cfg(unix)]
pub use private_log::{
    FreshReportSlot, ProcessEvidenceFailure, ProcessEvidenceFailureKind, ReportEvidenceFailure,
    ReportEvidenceFailureKind, ReportedProcessOutcome, prepare_fresh_report, run_process_recorded,
    run_process_recorded_with_report, write_private_log,
};
pub use process_runner::{ProcessOutcome, run_process, run_process_with_address_space_limit};
pub use process_spec::ProcessSpec;
#[cfg(unix)]
mod native_version_request;
#[cfg(unix)]
pub use native_version_request::NativeVersionRequest;
#[cfg(unix)]
mod native_version_observation;
#[cfg(unix)]
pub use native_version_observation::NativeVersionObservation;
#[cfg(unix)]
mod native_version_probe;
#[cfg(unix)]
pub use native_version_probe::observe_native_version;
pub use source_snapshot::SourceSnapshot;
#[cfg(unix)]
pub use task_file_lock::TaskFileLock;
pub use task_scheduler::{SchedulerError, TaskExecution, TaskOutcome, run_task_graph};
pub use termination::Termination;

/// 查询本进程是否收到 Ctrl-C；未安装处理器的平台保持 false。
#[must_use]
pub fn sigint_cancellation_requested() -> bool {
    #[cfg(unix)]
    {
        interrupt::sigint_cancelled()
    }
    #[cfg(not(unix))]
    {
        false
    }
}

/// 读取普通文件，拒绝符号链接和过大输入；静态观察不执行项目脚本。
pub fn read_bounded_regular_file(path: &Path, max_bytes: u64) -> io::Result<Vec<u8>> {
    #[cfg(unix)]
    let file = fs::OpenOptions::new()
        .read(true)
        // 非阻塞打开后再核对类型，FIFO 无写端时也不得卡住配置/清单检查。
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC)
        .open(path)?;
    #[cfg(not(unix))]
    let file = fs::File::open(path)?;
    let metadata = file.metadata()?;
    if !metadata.file_type().is_file() || metadata.len() > max_bytes {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "输入不是有界的普通文件",
        ));
    }
    let mut content = Vec::new();
    file.take(max_bytes.saturating_add(1))
        .read_to_end(&mut content)?;
    if content.len() as u64 > max_bytes {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "输入超过大小上限",
        ));
    }
    Ok(content)
}

#[cfg(test)]
mod tests {
    use super::read_bounded_regular_file;
    use std::io::ErrorKind;
    use std::path::Path;

    #[test]
    fn missing_file_is_not_a_clean_observation() {
        let error = read_bounded_regular_file(Path::new("/nonexistent/codeguard-test-file"), 1)
            .expect_err("不存在的文件不能被观察为成功");
        assert_eq!(error.kind(), ErrorKind::NotFound);
    }

    #[cfg(unix)]
    #[test]
    fn bounded_reader_rejects_symlink_and_oversized_file() {
        use std::fs;
        use std::os::unix::fs::symlink;
        use std::sync::atomic::{AtomicU64, Ordering};

        static NEXT: AtomicU64 = AtomicU64::new(0);
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("codeguard-read-{}-{id}", std::process::id()));
        fs::create_dir(&root).expect("fixture root");
        let file = root.join("package.json");
        let link = root.join("alias.json");
        fs::write(&file, br#"{"version":"1"}"#).expect("fixture file");
        symlink(&file, &link).expect("fixture link");
        assert!(read_bounded_regular_file(&link, 1024).is_err());
        assert_eq!(
            read_bounded_regular_file(&file, 1)
                .expect_err("oversized file")
                .kind(),
            ErrorKind::InvalidData
        );
        fs::remove_dir_all(root).expect("remove fixture");
    }
}

mod package_download_request;
pub use package_download_request::PackageDownloadRequest;
mod package_download;
pub use package_download::{
    download_verified_archive, download_verified_package, package_download_authority,
    validate_package_download_request,
};
#[cfg(test)]
mod download_test_server;

#[cfg(feature = "wasm-precheck")]
mod wasm_keyword_sequence;
#[cfg(feature = "wasm-precheck")]
pub use wasm_keyword_sequence::{WasmKeywordSequence, scan_wasm_keyword_sequence};

#[cfg(feature = "wasm-precheck")]
mod wasm_form_terminator;
#[cfg(feature = "wasm-precheck")]
pub use wasm_form_terminator::WasmFormTerminator;
#[cfg(feature = "wasm-precheck")]
mod wasm_form_terminator_scan;
#[cfg(feature = "wasm-precheck")]
pub use wasm_form_terminator_scan::scan_wasm_form_terminators;

#[cfg(feature = "wasm-precheck")]
mod wasm_outer_return;
#[cfg(feature = "wasm-precheck")]
pub use wasm_outer_return::WasmOuterReturn;
#[cfg(feature = "wasm-precheck")]
mod wasm_outer_return_scan;
#[cfg(feature = "wasm-precheck")]
pub use wasm_outer_return_scan::{WasmOuterReturnScan, scan_wasm_outer_returns};
