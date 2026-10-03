#![cfg(feature = "wasm-precheck")]

use codeguard_cli::grammar_route::route_source;
use std::collections::BTreeSet;

#[test]
fn routes_every_pinned_grammar_without_merging_dialects() {
    let cases = [
        ("arkts", "a.ets"),
        ("c", "a.c"),
        ("cfml", "a.cfm"),
        ("cobol", "a.cbl"),
        ("cpp", "a.cpp"),
        ("csharp", "a.cs"),
        ("dart", "a.dart"),
        ("erlang", "a.erl"),
        ("go", "a.go"),
        ("java", "a.java"),
        ("javascript", "a.js"),
        ("kotlin", "a.kt"),
        ("lua", "a.lua"),
        ("luau", "a.luau"),
        ("nix", "a.nix"),
        ("objc", "a.m"),
        ("pascal", "a.pas"),
        ("php", "a.php"),
        ("python", "a.py"),
        ("r", "a.r"),
        ("ruby", "a.rb"),
        ("rust", "a.rs"),
        ("scala", "a.scala"),
        ("solidity", "a.sol"),
        ("swift", "a.swift"),
        ("terraform", "a.tf"),
        ("tsx", "a.tsx"),
        ("typescript", "a.ts"),
        ("vbnet", "a.vb"),
        ("zig", "a.zig"),
    ];
    let mut observed = BTreeSet::new();
    for (language, path) in cases {
        let source: &[u8] = if path == "a.m" {
            b"@interface Foo : NSObject\n@end\n"
        } else {
            b"source"
        };
        let routes = route_source(path, source);
        assert_eq!(routes.len(), 1, "{path}");
        assert_eq!(routes[0].language, language, "{path}");
        observed.insert(language);
    }
    let cfscript = route_source("component.cfs", b"component {}");
    assert_eq!(cfscript[0].language, "cfscript");
    observed.insert("cfscript");
    let cfml = route_source(
        "page.cfm",
        b"<cfquery name=\"q\">SELECT id FROM users</cfquery>",
    );
    assert!(cfml.iter().any(|route| route.language == "cfquery"));
    observed.insert("cfquery");
    assert_eq!(observed.len(), 32);
}

#[test]
fn ambiguous_and_embedded_sources_do_not_get_speculative_routes() {
    assert!(route_source("a.h", b"int x;").is_empty());
    assert!(route_source("a.sql", b"SELECT 1").is_empty());
    assert!(route_source("plot.m", b"x = [1, 2, 3];\nplot(x);\n").is_empty());
    assert!(
        route_source(
            "plot.m",
            b"title('@interface Foo');\n% #import is only an example\n"
        )
        .is_empty()
    );
    assert!(route_source("synth.sc", b"{ SinOsc.ar(440) }.play;\n").is_empty());
    assert_eq!(
        route_source("model.m", b"@implementation Model\n@end\n")[0].language,
        "objc"
    );
    assert_eq!(
        route_source("script.scala", b"object App {}\n")[0].language,
        "scala"
    );
    assert!(
        route_source("a.cfm", b"<cfquery>SELECT 1")
            .iter()
            .all(|r| r.language != "cfquery")
    );
    let routes = route_source("a.tsx", b"const C = () => <div />;");
    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].language, "tsx");
}

#[test]
fn cfquery_route_keeps_html_comment_tags_but_skips_nested_cfml_comments() {
    let source = b"<!-- <cfquery>SELECT #html# FROM users</cfquery> -->\n<!--- outer <!--- inner ---> <cfquery>SELECT #broken</cfquery> --->\n<cfquery name=\"real\">SELECT #id# FROM users</cfquery>";
    let routes = route_source("page.cfm", source);
    let query_routes: Vec<_> = routes
        .iter()
        .filter(|route| route.language == "cfquery")
        .collect();
    assert_eq!(query_routes.len(), 2);
    assert_eq!(query_routes[0].source, b"SELECT #html# FROM users");
    assert_eq!(query_routes[1].source, b"SELECT #id# FROM users");
    assert_eq!(
        &source[query_routes[1].byte_offset
            ..query_routes[1].byte_offset + query_routes[1].source.len()],
        query_routes[1].source
    );
}

#[test]
fn cfquery_route_uses_quote_aware_tag_boundary_and_skips_non_markup_contexts() {
    let source = b"<cfset text=\"<cfquery>SELECT #bad</cfquery>\">\n<cfscript>var text = \"<cfquery>SELECT #bad</cfquery>\";</cfscript>\n<cfquery name=\"a>b\">SELECT #id# FROM users</cfquery>";
    let routes = route_source("page.cfm", source);
    let queries: Vec<_> = routes
        .iter()
        .filter(|route| route.language == "cfquery")
        .collect();
    assert_eq!(queries.len(), 1);
    assert_eq!(queries[0].source, b"SELECT #id# FROM users");
}

#[test]
fn cfquery_route_ignores_nested_cfml_comment_inside_open_tag() {
    let source = b"<cfquery <!--- note > <!--- nested ---> more ---> name=\"q\">SELECT #id# FROM users</cfquery>";
    let routes = route_source("page.cfm", source);
    let query = routes
        .iter()
        .find(|route| route.language == "cfquery")
        .unwrap();
    assert_eq!(query.source, b"SELECT #id# FROM users");
}
