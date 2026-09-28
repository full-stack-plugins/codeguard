use codeguard_adapters::MavenModuleModel;

#[test]
fn model_retains_direct_declarations_and_marks_dynamic_inputs_unresolved() {
    let model = MavenModuleModel::observe(br#"<project xmlns="http://maven.apache.org/POM/4.0.0"><groupId>example</groupId><artifactId>api</artifactId><version>1</version><modules><module>child</module></modules><dependencies><dependency><groupId>example</groupId><artifactId>core</artifactId><version>1</version></dependency><dependency><groupId>example</groupId><artifactId>managed</artifactId><version>${version}</version></dependency></dependencies><profiles><profile><modules><module>profile-only</module></modules></profile></profiles><parent/><dependencyManagement/></project>"#).unwrap();
    assert_eq!(
        model.coordinates,
        Some(("example".into(), "api".into(), "1".into()))
    );
    assert_eq!(model.modules, ["child"]);
    assert_eq!(
        model.dependencies,
        [(
            "example".into(),
            "core".into(),
            "1".into(),
            "compile".into()
        )]
    );
    for reason in [
        "profiles_not_evaluated",
        "parent_not_evaluated",
        "dependencyManagement_not_evaluated",
        "dependency_coordinates_unresolved",
    ] {
        assert!(model.unresolved.contains(reason));
    }
}

#[test]
fn malformed_and_ambiguous_coordinates_are_not_guessed() {
    for bytes in [
        b"<project".as_slice(),
        b"<other/>",
        b"<project xmlns='urn:fake'/>",
    ] {
        assert!(MavenModuleModel::observe(bytes).is_err());
    }
    assert!(MavenModuleModel::observe(&vec![b' '; 256 * 1024 + 1]).is_err());
    for xml in [
        "<project><groupId>a</groupId><artifactId>b</artifactId><version>1</version><version>2</version></project>",
        "<project><groupId>a</groupId><artifactId>${id}</artifactId><version>1</version></project>",
        "<project><groupId>a b</groupId><artifactId>c</artifactId><version>1</version></project>",
        "<project><groupId>a</groupId><artifactId><nested>b</nested></artifactId><version>1</version></project>",
    ] {
        let model = MavenModuleModel::observe(xml.as_bytes()).unwrap();
        assert!(model.coordinates.is_none());
        assert!(model.unresolved.contains("project_coordinates_unresolved"));
    }
}

#[test]
fn conditional_or_special_dependencies_do_not_look_like_ordinary_local_dependencies() {
    for special in [
        "<optional>true</optional>",
        "<scope>${scope}</scope>",
        "<classifier>tests</classifier>",
        "<type>test-jar</type>",
        "<systemPath>/tmp/library.jar</systemPath>",
    ] {
        let xml = format!(
            "<project><dependencies><dependency><groupId>a</groupId><artifactId>b</artifactId><version>1</version>{special}</dependency></dependencies></project>"
        );
        let model = MavenModuleModel::observe(xml.as_bytes()).unwrap();
        assert!(model.dependencies.is_empty());
        assert!(
            model
                .unresolved
                .contains("dependency_attributes_unresolved")
        );
    }
}

#[test]
fn segmented_xml_text_cannot_be_truncated_into_a_different_module_identity() {
    let model=MavenModuleModel::observe(b"<project><groupId>example</groupId><artifactId>api<!-- split -->extra</artifactId><version>1</version><modules><module>api<!-- split -->/other</module></modules></project>").unwrap();
    assert!(model.coordinates.is_none());
    assert!(model.modules.is_empty());
    assert!(model.unresolved.contains("module_declaration_unresolved"));
}
