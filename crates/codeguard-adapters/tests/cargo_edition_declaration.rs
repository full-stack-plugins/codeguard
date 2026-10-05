use codeguard_adapters::CargoEditionDeclaration;

#[test]
fn editions_default_and_explicit_package_override_are_distinct() {
    for edition in ["2015", "2018", "2021", "2024"] {
        let package = format!("[package]\nname='sample'\nedition='{edition}'\n");
        let parsed = CargoEditionDeclaration::observe(package.as_bytes()).unwrap();
        assert_eq!(parsed.resolve(None).unwrap(), edition);
        assert_eq!(
            parsed
                .resolve(Some(b"[workspace.package]\nedition='2024'\n"))
                .unwrap(),
            edition
        );
    }
    let parsed = CargoEditionDeclaration::observe(b"[package]\nname='sample'\n").unwrap();
    assert_eq!(parsed.resolve(None).unwrap(), "2015");
    assert!(!parsed.inherits_workspace);
}

#[test]
fn inheritance_is_explicit_and_requires_a_valid_provider() {
    let parsed = CargoEditionDeclaration::observe(
        b"[package]\nname='sample'\nedition.workspace=true\nworkspace='../root'\n",
    )
    .unwrap();
    assert!(parsed.inherits_workspace);
    assert_eq!(parsed.workspace_locator.as_deref(), Some("../root"));
    assert!(parsed.resolve(None).is_err());
    assert!(parsed.resolve(Some(b"[workspace]\n")).is_err());
    assert!(
        parsed
            .resolve(Some(b"[package]\nedition='2021'\n"))
            .is_err()
    );
    assert_eq!(
        parsed
            .resolve(Some(b"[workspace.package]\nedition='2021'\n"))
            .unwrap(),
        "2021"
    );
    assert!(
        parsed
            .resolve(Some(b"[workspace.package]\nedition='2030'\n"))
            .is_err()
    );
}

#[test]
fn invalid_ambiguous_and_virtual_inputs_never_choose_an_edition() {
    for input in [
        "",
        "[workspace]\n",
        "[package]\nedition='2030'\n",
        "[package]\nedition=2024\n",
        "[package]\nedition.workspace=false\n",
        "[package]\nedition={workspace=true,other='2021'}\n",
        "[package]\nedition='2021'\nedition='2024'\n",
        "[package]\nworkspace=42\n",
        "[package]\nworkspace=''\n",
        "[package]\nworkspace='../root'\n[workspace]\n",
    ] {
        assert!(
            CargoEditionDeclaration::observe(input.as_bytes()).is_err(),
            "{input}"
        );
    }
    assert!(CargoEditionDeclaration::observe(&[0xff]).is_err());
    assert!(CargoEditionDeclaration::observe(&vec![b' '; 256 * 1024 + 1]).is_err());
}
