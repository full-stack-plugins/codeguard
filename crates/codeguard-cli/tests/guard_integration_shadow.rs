use codeguard_cli::guard_integration::shadow::compare;
#[test]
fn native_only_never_supplies_an_engine_decision() {
    for raw in [b"unsupported native output".as_slice(), b"{}".as_slice()] {
        let comparison = compare(raw, None).unwrap();
        assert!(comparison.engine_report().is_err());
        assert_eq!(comparison.native_sha256().len(), 64);
    }
    assert!(compare(&vec![0; 1_048_577], None).is_err());
}
