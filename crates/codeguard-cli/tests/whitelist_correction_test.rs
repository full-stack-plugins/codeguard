//! 白名单纠错测试

use codeguard_cli::whitelist_correction::*;

#[test]
fn create_correction() {
    let record = WhitelistCorrection::create("c1", CorrectionStatus::Candidate);
    assert_eq!(record.id, "c1");
    assert_eq!(record.status, CorrectionStatus::Candidate);
}

#[test]
fn validate_no_cycle() {
    let records = vec![
        WhitelistCorrection::create("c1", CorrectionStatus::Candidate),
        WhitelistCorrection::create("c2", CorrectionStatus::Approved),
    ];
    
    assert!(WhitelistCorrection::validate_no_cycle(&records));
}

#[test]
fn validate_no_disguise() {
    let record = WhitelistCorrection::create("c1", CorrectionStatus::Approved);
    assert!(WhitelistCorrection::validate_no_disguise(&record));
}
