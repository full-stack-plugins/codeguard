use codeguard_adapters::CheckstyleCommand;
use std::path::PathBuf;

fn command() -> CheckstyleCommand {
    CheckstyleCommand {
        jar: PathBuf::from("/private/tool/checkstyle-all.jar"),
        config: PathBuf::from("/private/run/checkstyle.xml"),
        source: PathBuf::from("/private/run/source with spaces/Foo.java"),
        report: PathBuf::from("/private/run/report.xml"),
    }
}

#[test]
fn command_uses_native_jar_and_original_config_with_literal_arguments() {
    let args = command().args().unwrap();
    let actual: Vec<_> = args.iter().map(|a| a.to_str().unwrap()).collect();
    assert_eq!(
        actual,
        [
            "-jar",
            "/private/tool/checkstyle-all.jar",
            "-c",
            "/private/run/checkstyle.xml",
            "-f",
            "xml",
            "-o",
            "/private/run/report.xml",
            "/private/run/source with spaces/Foo.java"
        ]
    );
}

#[test]
fn ambiguous_paths_and_output_input_collisions_are_rejected() {
    for path in [
        "relative.jar",
        "/tmp/../tool.jar",
        "/tmp/tool\n.jar",
        "/tmp/tool\0.jar",
    ] {
        let mut input = command();
        input.jar = PathBuf::from(path);
        assert!(input.args().is_err(), "{path:?}");
    }
    for field in 0..3 {
        let mut input = command();
        input.report = match field {
            0 => input.jar.clone(),
            1 => input.config.clone(),
            _ => input.source.clone(),
        };
        assert!(input.args().is_err());
    }
    let mut input = command();
    input.config = input.source.clone();
    assert!(input.args().is_err());
}
