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
        let routes = route_source(path, b"source");
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
    assert!(
        route_source("a.cfm", b"<cfquery>SELECT 1")
            .iter()
            .all(|r| r.language != "cfquery")
    );
    let routes = route_source("a.tsx", b"const C = () => <div />;");
    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].language, "tsx");
}
