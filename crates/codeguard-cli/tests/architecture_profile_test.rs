//! 架构画像测试

use codeguard_cli::architecture_profile::*;

#[test]
fn domain_controller_not_auto_ddd() {
    let profile = ArchitectureProfiler::profile(true, true);
    
    // domain/controller 命名不自动认定 DDD
    assert!(!profile.auto_detected);
    assert_eq!(profile.dimension, ArchitectureDimension::Unknown);
}

#[test]
fn conflict_retained() {
    let profile = ArchitectureProfiler::profile(true, false);
    assert!(ArchitectureProfiler::validate_conflict_retained(&profile));
}

#[test]
fn inference_no_blocking() {
    let profile = ArchitectureProfiler::profile(true, true);
    assert!(ArchitectureProfiler::validate_inference_no_blocking(&profile));
}
