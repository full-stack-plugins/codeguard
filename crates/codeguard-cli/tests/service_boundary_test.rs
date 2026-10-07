//! 服务边界测试

use codeguard_cli::service_boundary::*;

#[test]
fn check_service_boundary() {
    let ports = vec![PortType::Observation, PortType::Policy];
    let boundary = ServiceBoundaryChecker::check("core", &ports);
    
    assert_eq!(boundary.service_name, "core");
    assert_eq!(boundary.ports.len(), 2);
}

#[test]
fn validate_shared_service() {
    let ports = vec![PortType::Check];
    let boundary = ServiceBoundaryChecker::check("core", &ports);
    assert!(ServiceBoundaryChecker::validate_shared_service(&boundary));
}

#[test]
fn validate_no_infrastructure() {
    let ports = vec![PortType::Storage];
    let boundary = ServiceBoundaryChecker::check("core", &ports);
    assert!(ServiceBoundaryChecker::validate_no_infrastructure(&boundary));
}
