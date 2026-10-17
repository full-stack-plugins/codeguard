//! 项目边界测试

use codeguard_cli::project_boundary::*;

fn create_root(path: &str, build_system: &str, language: &str) -> BuildRoot {
    BuildRoot {
        path: path.into(),
        build_system: build_system.into(),
        language: language.into(),
    }
}

#[test]
fn create_monorepo_boundary() {
    let roots = vec![
        create_root("/project/pkg-a", "maven", "java"),
        create_root("/project/pkg-b", "npm", "typescript"),
    ];
    
    let boundary = BoundaryChecker::create("/project", roots);
    assert!(boundary.is_monorepo);
}

#[test]
fn check_out_of_bounds() {
    let roots = vec![create_root("/project", "maven", "java")];
    let boundary = BoundaryChecker::create("/project", roots);
    
    assert!(BoundaryChecker::check_out_of_bounds(&boundary, "/other/file.rs"));
    assert!(!BoundaryChecker::check_out_of_bounds(&boundary, "/project/file.rs"));
}

#[test]
fn check_overlap() {
    let roots = vec![
        create_root("/project", "maven", "java"),
        create_root("/project/sub", "npm", "typescript"),
    ];
    
    let boundary = BoundaryChecker::create("/project", roots);
    assert!(BoundaryChecker::check_overlap(&boundary));
}

#[test]
fn no_overlap() {
    let roots = vec![
        create_root("/project/pkg-a", "maven", "java"),
        create_root("/project/pkg-b", "npm", "typescript"),
    ];
    
    let boundary = BoundaryChecker::create("/project", roots);
    assert!(!BoundaryChecker::check_overlap(&boundary));
}
