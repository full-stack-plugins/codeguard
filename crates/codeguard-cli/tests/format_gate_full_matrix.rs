//! 54 种 stable 语言的格式门禁端到端矩阵。
//!
//! 对每种 stable 语言真实执行 `codeguard format check`，断言：
//! 1. 工具可用时给出确定性判定（合规 allow / 不合规 deny）
//! 2. 工具缺失时**诚实降级**（formatter_not_found → incomplete → 退出 3），
//!    绝不静默签发 allow
//! 3. 语法损坏的文件不得被误判为"不合规"，也不得被误判为"合规"
//!
//! 本测试的价值在于：它不假设任何工具已安装，因此在 CI 与开发者本机
//! 都能验证「门禁语义正确」而非「恰好这台机器全绿」。

use std::path::PathBuf;
use std::process::Command;

fn ensure_clean_dir(name: &str) -> PathBuf {
    let tmp = std::env::temp_dir().join(format!("{}-{}", name, std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    tmp
}

/// 每种语言一个真实源文件样本（语法合法，缩进刻意不统一以便触发重排）。
struct Sample {
    language: &'static str,
    file: &'static str,
    body: &'static str,
}

const SAMPLES: &[Sample] = &[
    Sample { language: "java", file: "A.java", body: "class A {\n  int x=1;\n}\n" },
    Sample { language: "rust", file: "a.rs", body: "fn main(){\nlet x=1;\nprintln!(\"{}\",x);\n}\n" },
    Sample { language: "typescript", file: "a.ts", body: "const  x   =  1\n" },
    Sample { language: "python", file: "a.py", body: "x   =   1\n" },
    Sample { language: "go", file: "a.go", body: "package main\n\nfunc  main( ) {\n}\n" },
    Sample { language: "csharp", file: "A.cs", body: "class A {\n  int  x = 1;\n}\n" },
    Sample { language: "kotlin", file: "A.kt", body: "class A {\n  val  x  =  1\n}\n" },
    Sample { language: "swift", file: "a.swift", body: "let  x  =  1\n" },
    Sample { language: "php", file: "a.php", body: "<?php\n$x   =   1;\n" },
    Sample { language: "ruby", file: "a.rb", body: "x   =   1\n" },
    Sample { language: "scala", file: "A.scala", body: "object  A  {\n  val  x  =  1\n}\n" },
    Sample { language: "shell", file: "a.sh", body: "#!/bin/sh\nif [ $x = 1 ];then\necho  hi\nfi\n" },
    Sample { language: "dockerfile", file: "a.dockerfile", body: "FROM  alpine\nRUN  echo   hi\n" },
    Sample { language: "yaml", file: "a.yml", body: "key:   value\nlist:\n  -  a\n" },
    Sample { language: "elixir", file: "a.ex", body: "defmodule  A  do\n  def  run, do: :ok\nend\n" },
    Sample { language: "css", file: "a.css", body: "body{color:red;margin:0}\n" },
    Sample { language: "c", file: "a.c", body: "int  main( ) {\n  return  0;\n}\n" },
    Sample { language: "cpp", file: "a.cpp", body: "int  main( ) {\n  return  0;\n}\n" },
    Sample { language: "objc", file: "a.m", body: "int  main( ) {\n  return  0;\n}\n" },
    Sample { language: "dart", file: "a.dart", body: "void  main( )  {\n  var  x  =  1;\n}\n" },
    Sample { language: "vue", file: "a.vue", body: "<template>\n  <div  >x</div>\n</template>\n" },
    Sample { language: "svelte", file: "a.svelte", body: "<script>\n  let  x  =  1;\n</script>\n" },
    Sample { language: "astro", file: "a.astro", body: "---\nconst  x  =  1;\n---\n<div  >x</div>\n" },
    Sample { language: "solidity", file: "a.sol", body: "pragma  solidity ^0.8.0;\ncontract  A  { }\n" },
    Sample { language: "terraform", file: "a.tf", body: "resource   \"null_resource\"   \"a\"  {\n}\n" },
    Sample { language: "nix", file: "a.nix", body: "{  pkgs  }  :  [  pkgs.hello  ]\n" },
    Sample { language: "html", file: "a.html", body: "<div  >x</div>\n" },
    Sample { language: "sql", file: "a.sql", body: "SELECT   1;\n" },
    Sample { language: "graphql", file: "a.graphql", body: "query  Q  {\n  a\n}\n" },
    Sample { language: "protobuf", file: "a.proto", body: "syntax  =  \"proto3\";\nmessage  M  { }\n" },
    Sample { language: "markdown", file: "a.md", body: "#   Title\n\n\ntext\n" },
    Sample { language: "toml", file: "a.toml", body: "key   =   1\n" },
    Sample { language: "haskell", file: "A.hs", body: "module  Main  where\nmain  =  do\n  print  1\n" },
    Sample { language: "ocaml", file: "a.ml", body: "let  x  =  1\n" },
    Sample { language: "fsharp", file: "A.fs", body: "module  A\n\nlet  x  =  1\n" },
    Sample { language: "perl", file: "a.pl", body: "my  $x  =  1;\n" },
    Sample { language: "groovy", file: "A.groovy", body: "class  A  {\n  def  x  =  1\n}\n" },
    Sample { language: "clojure", file: "a.clj", body: "(ns  a)\n(def  x  1)\n" },
    Sample { language: "powershell", file: "a.ps1", body: "$x   =   1\n" },
    Sample { language: "zig", file: "a.zig", body: "const  x  =  1;\n" },
    Sample { language: "nim", file: "a.nim", body: "let  x  =  1\n" },
    Sample { language: "crystal", file: "a.cr", body: "x  =  1\n" },
    Sample { language: "julia", file: "a.jl", body: "x   =   1\n" },
    Sample { language: "elm", file: "a.elm", body: "module  A  exposing  (..)\nx  =  1\n" },
    Sample { language: "lua", file: "a.lua", body: "local  x  =  1\n" },
    Sample { language: "luau", file: "a.luau", body: "local  x  =  1\n" },
    Sample { language: "pascal", file: "a.pas", body: "program  A;\nbegin\nend.\n" },
    Sample { language: "r", file: "a.R", body: "x   <-   1\n" },
    Sample { language: "cfml", file: "a.cfc", body: "component  {\n}\n" },
    Sample { language: "vbnet", file: "A.vb", body: "Module  A\nEnd  Module\n" },
    Sample { language: "erlang", file: "a.erl", body: "-module(a).\n-export([f/0]).\nf()  ->  ok.\n" },
    Sample { language: "cuda", file: "a.cu", body: "__global__  void  k( )  { }\n" },
    Sample {
        language: "metal",
        file: "a.metal",
        body: "#include <metal_stdlib>\nusing namespace metal;\nkernel void k( device float *o [[buffer(0)]], uint g [[thread_position_in_grid]] )  {\n  o[g] = 1.0f;\n}\n",
    },
    Sample { language: "liquid", file: "a.liquid", body: "{%% assign  x  =  1 %%}\n{{  x  }}\n" },
    Sample {
        language: "arkts",
        file: "Index.ets",
        body: "@Entry\n@Component\nstruct   Index   {\n  @State   message:string='hi';\n  build( )  { }\n}\n",
    },
    // 路由条目：ansible 无专属格式化器，归口 yaml 通道执行
    Sample { language: "ansible", file: "playbook.yml", body: "---\n- name:   A\n  hosts:   all\n  tasks:  []\n" },
];

fn run_check(language: &str, dir: &PathBuf) -> (i32, serde_json::Value) {
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["format", "check", language, dir.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard format check");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value =
        serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("{language} 报告非 JSON: {stdout} ({e})"));
    (output.status.code().unwrap_or(-1), report)
}

/// 每种 stable 语言的 `format check` 必须给出可判定结果，工具缺失时诚实降级。
///
/// 关键不变量：**任何情况下都不得静默签发 allow**——
/// 工具缺失必须走 incomplete/退出 3，而不是因为"没检查到"就放行。
#[test]
fn all_stable_languages_produce_verifiable_gate_decisions() {
    let scope: serde_json::Value = serde_json::from_str(include_str!(
        "../../../rulepacks/format_gate_scope.json"
    ))
    .expect("format_gate_scope.json 应可解析");
    let stable: Vec<&str> = scope["integrated_languages"]
        .as_array()
        .expect("已接入集合")
        .iter()
        .filter_map(|v| v.as_str())
        .collect();

    // 样本必须覆盖全部 stable 语言，不允许测试自身漏覆盖
    let covered: Vec<&str> = SAMPLES.iter().map(|s| s.language).collect();
    for language in &stable {
        assert!(
            covered.contains(language),
            "{language} 是已接入的 stable 语言但端到端样本缺失——门禁语义将无人验证"
        );
    }
    // 覆盖全部已接入语言（含 stable 与可用工具链的 planned），数量跟随口径声明
    assert_eq!(
        stable.len() as u64,
        scope["integrated_count"].as_u64().expect("integrated_count"),
        "样本覆盖数必须等于口径声明的已接入数"
    );
    assert!(
        stable.len() as u64 >= 54,
        "全部 54 种 stable 语言必须纳入矩阵覆盖"
    );

    let mut with_tool = 0usize;
    let mut degraded = 0usize;

    for language in stable {
        let sample = SAMPLES
            .iter()
            .find(|s| s.language == language)
            .expect("样本应存在");
        let dir = ensure_clean_dir(&format!("cg-matrix-{language}"));
        std::fs::write(dir.join(sample.file), sample.body).unwrap();

        let (code, report) = run_check(language, &dir);
        let decision = report["delivery_decision"].as_str().unwrap_or("");
        let language_status = report["languages"]
            .as_array()
            .and_then(|rows| rows.first())
            .and_then(|row| row["status"].as_str())
            .unwrap_or("");

        // 核心不变量：缺工具 / 无源文件 / 工具故障都不得签发 allow
        if language_status != "complete" {
            assert_ne!(
                decision, "allow",
                "{language} 未完成检查（状态 {language_status}）却签发 allow——这是静默放行"
            );
            assert_eq!(
                code, 3,
                "{language} 降级时退出码应为 3（未完成），实际 {code}"
            );
            assert_eq!(
                report["command_status"], "incomplete",
                "{language} 降级时 command_status 应为 incomplete"
            );
            degraded += 1;
        } else {
            // 工具可用：必须是确定性判定
            assert!(
                matches!(decision, "allow" | "deny"),
                "{language} 完成检查时 decision 应为 allow/deny，实际 {decision:?}"
            );
            assert_eq!(
                code,
                if decision == "allow" { 0 } else { 1 },
                "{language} 的 decision={decision} 与退出码 {code} 不一致"
            );
            // 不合规文件必须被列出，且不得混入 failed
            if decision == "deny" {
                assert!(
                    report["unformatted_count"].as_u64().unwrap_or(0) > 0,
                    "{language} 判 deny 但未列出任何不合规文件"
                );
            }
            with_tool += 1;
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    // 本机至少要有若干真实工具跑通，否则测试等于只验证了降级路径
    assert!(
        with_tool > 0,
        "没有任何语言完成真实检查，测试只覆盖了降级路径——无法证明门禁可执行"
    );
    eprintln!("真实执行 {with_tool} 种，降级 {degraded} 种");
}

/// 缺依赖场景必须诚实降级，绝不静默放行。
///
/// 用一个绝对不存在的工具路径绑定，模拟「工具未安装/依赖缺失」。
#[test]
fn missing_tool_never_silently_allows() {
    // 必须按各语言档案里的真实 tool_key 绑定不存在的路径，
    // 否则绑定名不匹配会回退到 PATH 查找，测不到「显式工具缺失」这条路径。
    let inventory: serde_json::Value = serde_json::from_slice(
        &Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["format", "list", "--format=json"])
            .output()
            .expect("无法运行 codeguard format list")
            .stdout,
    )
    .expect("JSON 解析失败");

    let mut checked = 0usize;
    for sample in SAMPLES.iter().take(8) {
        let language_name = sample.language;
        let dir = ensure_clean_dir(&format!("cg-missing-{language_name}"));
        std::fs::write(dir.join(sample.file), sample.body).unwrap();

        // 路由条目按其目标通道的 tool_key 绑定
        let row = inventory["languages"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["language"] == sample.language)
            .expect("档案应含该语言");
        let tool_key = row["tool_key"].as_str().expect("档案应声明 tool_key");
        let binding = format!("{tool_key}=/nonexistent/definitely-not-a-real-tool");

        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "format",
                "check",
                sample.language,
                dir.to_str().unwrap(),
                "--tool",
                &binding,
                "--format=json",
            ])
            .output()
            .expect("无法运行 codeguard format check");
        let stdout = String::from_utf8_lossy(&output.stdout);

        // 命令要么给出未接入（退出 3），要么给出 JSON 报告；
        // 无论哪条路径都不得退出 0
        let code = output.status.code().unwrap_or(-1);
        assert_ne!(
            code, 0,
            "{} 在工具缺失时退出 0——这是静默放行",
            sample.language
        );

        if let Ok(report) = serde_json::from_str::<serde_json::Value>(&stdout) {
            assert_ne!(
                report["delivery_decision"].as_str().unwrap_or(""),
                "allow",
                "{} 工具缺失却签发 allow",
                sample.language
            );
            // 缺工具应被明确归因为 formatter_not_found（或该语言未接入）
            if let Some(row) = report["languages"].as_array().and_then(|r| r.first()) {
                let status = row["status"].as_str().unwrap_or("");
                let reason = row["reason"].as_str().unwrap_or("");
                assert!(
                    status == "incomplete" || status == "no_files",
                    "{} 工具缺失时语言状态应为 incomplete/no_files，实际 {status}",
                    sample.language
                );
                if status == "incomplete" && !reason.is_empty() {
                    assert!(
                        reason.contains("formatter_not_found")
                            || reason.contains("formatter_execution_failed"),
                        "{} 缺工具原因应可归因，实际 {reason}",
                        sample.language
                    );
                }
            }
            checked += 1;
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
    assert!(checked > 0, "缺工具场景应至少产出一份可解析报告");
}

/// 路由条目 ansible 必须真正执行（而非仅声明缺口），并在 all 模式下不重复跑。
#[test]
fn routed_ansible_entry_executes_real_gate() {
    let dir = ensure_clean_dir("cg-routed-ansible");
    std::fs::write(
        dir.join("playbook.yml"),
        "---\n- name:   A\n  hosts:   all\n  tasks:  []\n",
    )
    .unwrap();

    let (_, report) = run_check("ansible", &dir);
    let rows = report["languages"].as_array().expect("应有语言明细");
    let row = rows.first().expect("应有执行语言");

    // ansible 是路由条目：报告须披露路由关系，且目标通道是 yaml
    assert_eq!(
        row["routed_from"], "ansible",
        "ansible 报告应标注 routed_from=ansible"
    );
    assert_eq!(
        row["routed_to"], "yaml",
        "ansible 应路由到 yaml 通道执行"
    );
    assert_eq!(
        row["formatter"], "prettier",
        "ansible 路由目标应使用 yaml 通道的格式化器"
    );
    // 路由后必须真的执行检查（不是 not_integrated）
    assert_ne!(
        row["status"], "not_integrated",
        "ansible 已改为可执行路由条目，不应再报未接入"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// `format check all` 不得把路由条目与其目标通道重复执行。
#[test]
fn format_all_does_not_duplicate_routed_entries() {
    let dir = ensure_clean_dir("cg-all-routed");
    std::fs::write(dir.join("a.yml"), "key:   value\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["format", "check", "all", dir.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard format check all");
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).expect("JSON 解析失败");

    let languages: Vec<&str> = report["languages"]
        .as_array()
        .expect("应有语言明细")
        .iter()
        .filter_map(|r| r["language"].as_str())
        .collect();

    // .yml 属 yaml 通道；all 模式不应再跑 ansible（否则同一批文件格式化两次）
    assert!(
        languages.contains(&"yaml"),
        "all 模式应包含 yaml 通道以处理 .yml 文件"
    );
    assert!(
        !languages.contains(&"ansible"),
        "all 模式不应重复执行路由条目 ansible（目标通道 yaml 已覆盖同一批文件），实际执行了 {languages:?}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// 解析失败检测：clang-format 对无法解析的源文件会静默退 0，
/// 运行时必须用第二遍探测（--output-replacements-xml）把它降级为未完成。
///
/// 这不是假设——是实测到的真实缺陷：clang-format 遇到解析失败会原样输出并返回成功，
/// 于是损坏的 C/C++/CUDA/proto/ObjC/Metal 源码会被误报为「格式合规」。
#[test]
fn unparsable_source_is_never_reported_as_compliant() {
    let clang_format = std::env::var("CODEGUARD_TEST_CLANG_FORMAT")
        .ok()
        .or_else(|| which("clang-format"));
    let Some(clang_format) = clang_format else {
        eprintln!("clang-format 不可用，跳过解析失败检测断言");
        return;
    };

    // clang-format 系语言各测一个损坏样本
    for (language, file, body) in [
        ("c", "broken.c", "int main( {\n  return 0;\n}\n"),
        ("cpp", "broken.cpp", "int main( {\n  return 0;\n}\n"),
        ("metal", "broken.metal", "#include <metal_stdlib>\nkernel void k( {\n"),
        ("cuda", "broken.cu", "__global__ void k( {\n"),
    ] {
        let dir = ensure_clean_dir(&format!("cg-unparsable-{language}"));
        std::fs::write(dir.join(file), body).unwrap();

        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "format",
                "check",
                language,
                dir.to_str().unwrap(),
                "--tool",
                &format!("clang-format={clang_format}"),
                "--format=json",
            ])
            .output()
            .expect("无法运行 codeguard format check");
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).expect("JSON 解析失败");

        // 核心断言：损坏文件绝不能被判为 allow
        assert_ne!(
            report["delivery_decision"].as_str().unwrap_or(""),
            "allow",
            "{language} 的语法损坏文件被判为格式合规——解析失败检测失效"
        );
        assert_eq!(
            report["exit_code"], 3,
            "{language} 解析失败应退出 3（未完成），实际 {}",
            report["exit_code"]
        );
        assert_eq!(
            output.status.code(),
            Some(3),
            "{language} 解析失败应退出 3"
        );

        // 且原因应可归因
        let reason = report["languages"][0]["reason"].as_str().unwrap_or("");
        assert!(
            reason == "source_not_parsable_by_formatter" || reason == "formatter_execution_failed",
            "{language} 解析失败原因应可归因，实际 {reason:?}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}

fn which(name: &str) -> Option<String> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
        .map(|p| p.to_string_lossy().to_string())
}
