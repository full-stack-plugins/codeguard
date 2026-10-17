//! 质量配置映射测试

use codeguard_cli::quality_config_mapping::*;

#[test]
fn map_prerequisites() {
    let prereqs = vec![
        Prerequisite {
            name: "jdk".into(),
            satisfied: true,
            required: true,
        },
    ];
    
    let mapping = QualityConfigMapper::map(prereqs);
    assert_eq!(mapping.prerequisites.len(), 1);
}

#[test]
fn validate_no_fabrication() {
    let mapping = QualityConfigMapper::map(vec![]);
    assert!(QualityConfigMapper::validate_no_fabrication(&mapping));
}

#[test]
fn validate_no_auto_exclude() {
    let mapping = QualityConfigMapper::map(vec![]);
    assert!(QualityConfigMapper::validate_no_auto_exclude(&mapping));
}
