#![cfg(unix)]

use codeguard_adapters::Pmd6Command;
use std::ffi::OsString;
use std::path::PathBuf;

#[test]
fn pmd6_command_binds_one_source_one_ruleset_and_one_private_report_path() {
    let command = Pmd6Command {
        source: PathBuf::from("/workspace/src/main/java/Bad_Name.java"),
        ruleset: "rulesets/java/ali-naming.xml".into(),
        report: PathBuf::from("/private/run-1/pmd.xml"),
    };
    let args = command.args().expect("固定 PMD 6 参数");
    assert_eq!(
        args,
        vec![
            OsString::from("pmd"),
            OsString::from("-d"),
            command.source.as_os_str().to_os_string(),
            OsString::from("-R"),
            OsString::from("rulesets/java/ali-naming.xml"),
            OsString::from("-f"),
            OsString::from("xml"),
            OsString::from("-r"),
            command.report.as_os_str().to_os_string(),
            OsString::from("-showsuppressed"),
            OsString::from("-no-cache"),
        ]
    );
}

#[test]
fn pmd6_command_rejects_ambiguous_paths_or_rulesets() {
    let mut command = Pmd6Command {
        source: PathBuf::from("/workspace/Bad.java"),
        ruleset: "rulesets/java/ali-naming.xml".into(),
        report: PathBuf::from("/private/run-1/pmd.xml"),
    };
    command.report = PathBuf::from("pmd.xml");
    assert!(command.args().is_err());
    command.report = PathBuf::from("/private/run-1/pmd.xml");
    command.source = PathBuf::from("Bad.java");
    assert!(command.args().is_err());
    command.source = PathBuf::from("/workspace/Bad.java");
    command.ruleset = "rulesets/java/ali-naming.xml,/other.xml".into();
    assert!(command.args().is_err());
    command.ruleset = "-R".into();
    assert!(command.args().is_err());
}
