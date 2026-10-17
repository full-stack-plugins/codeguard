//! RepairBrief 测试

use codeguard_cli::repair_brief::*;

#[test]
fn generate_repair_brief() {
    let brief = RepairBriefGenerator::generate(
        "main.rs",
        10,
        "unused_var",
        &["src/".to_string()],
    );
    
    assert_eq!(brief.target.file, "main.rs");
    assert_eq!(brief.target.line, 10);
    assert_eq!(brief.target.rule, "unused_var");
    assert!(!brief.scope.is_empty());
    assert!(!brief.steps.is_empty());
}

#[test]
fn instructions_not_executable() {
    let brief = RepairBriefGenerator::generate("main.rs", 10, "unused_var", &[]);
    assert!(RepairBriefGenerator::validate_instructions_not_executable(&brief));
}

#[test]
fn recheck_condition_present() {
    let brief = RepairBriefGenerator::generate("main.rs", 10, "unused_var", &[]);
    assert!(!brief.recheck_condition.is_empty());
}
