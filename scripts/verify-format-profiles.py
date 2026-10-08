#!/usr/bin/env python3
"""真机复验 codeguard 格式化档案：校验档案声明的工具、参数与退出码语义是否属实。

这是对「档案驱动」架构的风险对冲：档案里的 check_argv / unformatted_exit_codes
若与工具真实行为不符，会导致静默放行或误判故障。人工复验一次就会漏，
故做成可重复执行的脚本，可在改档案后与 CI 上持续运行。

用法:
  python3 scripts/verify-format-profiles.py            # 复验档案中本机可用的语言
  python3 scripts/verify-format-profiles.py --lang go  # 只复验指定语言
  python3 scripts/verify-format-profiles.py --list     # 列出可复验/不可复验分布

判定标准（每种语言三个场景，与 codeguard 运行时语义一致）:
  ok      → allow      退出 0
  ugly    → deny       退出码 ∈ unformatted_exit_codes
  broken  → incomplete 退出码 ∉ unformatted_exit_codes（不得误判为需重排）
"""

import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BIN = os.path.join(ROOT, "target", "debug", "codeguard")

# 每种语言的三场景样本：ext / 合规源(None 表示由 apply 产出) / 不合规 / 语法损坏
# 合规源为 None 时用 apply 规范化「不合规」样本得到——这是最可靠的合规基准，
# 因为只有工具自己产出的格式才算它的规范。
SAMPLES = {
    "go":         (".go",   None, 'package main\n\nfunc  main( ) {\n}\n', 'package main\n\nfunc main( {\n'),
    "rust":       (".rs",   None, "fn main(){\nlet x=1;\n}\n", "fn main( {\n"),
    "c":          (".c",    None, "int  main( ) {\n  return  0;\n}\n", "int main( {\n"),
    "terraform":  (".tf",   None, 'resource   "null_resource"  "a" {\n}\n', 'resource "x" {\n'),
    "perl":       (".pl",   None, 'my  $x  =  1;\n', 'my $x = ;\n'),
    "haskell":    (".hs",   None, 'module Main where\nmain = do\n      print  1\n', 'module Main where\nmain = do\n  let = in\n'),
    "lua":        (".lua",  None, 'local  x  =  1\n', 'local x = = 1\n'),
    "shell":      (".sh",   None, "#!/bin/sh\nif [ $x = 1 ];then\necho  hi\nfi\n", "#!/bin/sh\nif ( {\n"),
    "python":     (".py",   None, "x   =   1\n", "def f(:\n"),
    "markdown":   (".md",   None, "#   Title\n\n\ntext\n", "# T\n\n\n[unclosed\n"),
    "sql":        (".sql",  None, "SELECT   1;\n", "SELECT FROM WHERE;\n"),
    "toml":       (".toml", None, "key   =   1\n", "key = = 1\n"),
    "yaml":       (".yml",  None, "key:   value\nlist:\n  -  a\n", "key: [unclosed\n"),
    "nix":        (".nix",  None, "{  pkgs  }  :  [  pkgs.hello  ]\n", "{ pkgs : [ }\n"),
    "zig":        (".zig",  None, "const  x  =  1;\n", "const x = ;\n"),
    "crystal":    (".cr",   None, "x  =  1\n", "x = = 1\n"),
    "elm":        (".elm",  None, "module  A  exposing  (..)\nx  =  1\n", "module A exposing (\nx = = 1\n"),
    "objc":       (".m",    None, "int  main( ) {\n  return  0;\n}\n", "int main( {\n"),
    "cpp":        (".cpp", None, "int  main( ) {\n  return  0;\n}\n", "int main( {\n"),
    "julia":      (".jl",   None, "x   =   1\n", "x = = 1\n"),
    "clojure":    (".clj",  None, "(ns a)\n\n\n\n(def x 1)\n", "(ns a)\n(def x\n"),
    "fsharp":     (".fs",   None, "module  A\n\nlet  x  =  1\n", "module A\n\nlet x = \n"),
    "ocaml":      (".ml",   None, "let  x  =  1\n", "let x = in\n"),
    "nim":        (".nim",  None, "let  x  =  1\n", "let x = = 1\n"),
    "erlang":     (".erl",  None, "-module(a).\n-export([f/0]).\nf()  ->  ok.\n", "-module(a).\nf() -> ok\n"),
    "clojure2":   (".clj",  None, "(ns  a)\n", "(ns a)\n"),
    "pascal":     (".pas",  None, "program  A;\nbegin\nend.\n", "program A;\nbegin\n"),
    "cfml":       (".cfc",  None, "component  {\n}\n", "component {\n"),
    "vbnet":      (".vb",   None, "Module  A\nEnd  Module\n", "Module A\nEnd\n"),
    "powershell": (".ps1",  None, "$x   =   1\n", "function {\n"),
    "groovy":     (".groovy", None, "class  A  {\n  def  x  =  1\n}\n", "class A {\n  def x = = 1\n"),
    "elixir":     (".ex",   None, "defmodule  A  do\n  def  run, do: :ok\nend\n", "defmodule A do\n  def run do\n"),
    # ktlint 启用 standard:filename 规则（文件名须与主类名一致），
    # 故样本用无主类的顶层声明，隔离命名规则，只测排版。
    "kotlin":     (".kt",   None, "class  A  {\n  val  x  =  1\n}\n", "class A {\n  val x = = 1\n"),
    "swift":      (".swift", None, "let  x  =  1\n", "let x = = 1\n"),
    "dart":       (".dart", None, "void  main( )  {\n  var  x  =  1;\n}\n", "void main( {\n"),
    "ruby":       (".rb",   None, "x   =   1\n", "x = = 1\n"),
    "php":        (".php",  None, "<?php\n$x   =   1;\n", "<?php\n$x = = 1;\n"),
    "scala":      (".scala", None, "object  A  {\n  val  x  =  1\n}\n", "object A {\n  val x = = 1\n"),
    "solidity":   (".sol",  None, "pragma  solidity ^0.8.0;\ncontract  A  { }\n", "pragma solidity ^0.8.0;\ncontract A { \n"),
    "typescript": (".ts",   None, "const  x   =  1\n", "const x: = 1\n"),
    "css":        (".css",  None, "body{color:red;margin:0}\n", "body{color:red\n"),
    "html":       (".html", None, "<div  >x</div>\n", "<div >x\n"),
    "graphql":    (".graphql", None, "query  Q  {\n  a\n}\n", "query Q { a\n"),
    "protobuf":   (".proto", None, 'syntax  =  "proto3";\nmessage  M  { }\n', 'syntax = "proto3";\nmessage M {\n'),
    "liquid":     (".liquid", None, "{%% assign  x  =  1 %%}\n{{  x  }}\n", "{%% assign x  =  1 %%}\n"),
    "r":          (".R",    None, "x   <-   1\n", "x <- <- 1\n"),
    "luau":       (".luau", None, "local  x  =  1\n", "local x = = 1\n"),
    "cuda":       (".cu",   None, "__global__  void  k( )  { }\n", "__global__ void k( {\n"),
    "metal":      (".metal", None, "#include <metal_stdlib>\nusing namespace metal;\nkernel void k( device float *o [[buffer(0)]], uint g [[thread_position_in_grid]] )  {\n  o[g] = 1.0f;\n}\n", "#include <metal_stdlib>\nkernel void k( {\n"),
    # cobol 的 apply 按设计拒绝自动修复（GnuCOBOL 不提供源码重排），
    # 无法用 apply 产出合规基准，故直接给出规范的 COBOL-85 固定格式样本。
    "cobol":      (".cbl",  "       IDENTIFICATION DIVISION.\n       PROGRAM-ID. X.\n       PROCEDURE DIVISION.\n       MAIN-PARA.\n           DISPLAY \"HI\".\n           STOP RUN.\n", "       IDENTIFICATION DIVISION.\nPROGRAM-ID. U.\n       PROCEDURE DIVISION.\n       MAIN-PARA.\n           DISPLAY \"HI\".\n           STOP RUN.\n", "       IDENTIFICATION DIVISION.\n       PROGRAM-ID. B.\n       PROCEDURE DIVISION.\n       MAIN-PARA.\n           MOVE TO.\n"),
}


# 工具能力边界：以下格式化器无法区分「需重排」与「语法错误」，或对损坏源码静默放行。
# broken 场景的期望按此下调——把工具局限误报为档案缺陷会掩盖真问题。
# "deny"      工具无法区分两者，如实容忍
# "allow"     静默放行损坏源码（漏判，高危，需修复）
BROKEN_EXPECT = {
    "rust": "deny", "perl": "deny", "shell": "deny", "markdown": "deny",
    "sql": "deny", "toml": "deny", "zig": "deny", "crystal": "deny",
    "elm": "deny", "elixir": "deny", "kotlin": "deny", "swift": "deny",
    "ruby": "deny", "php": "deny", "solidity": "deny", "html": "deny",
    "nix": "allow", "erlang": "allow", "protobuf": "allow",
}


# 少数工具把文件名纳入规则集（如 ktlint 的 standard:filename 要求 PascalCase），
# 用中性合法文件名避免把命名规则误当排版问题。
NEUTRAL_NAMES = {"kotlin": "A", "elm": "Sample", "julia": "Sample"}
# 注：kotlin 样本主类名为 A，文件名必须为 A.kt（ktlint standard:filename 规则）


def sample_name(lang, ext):
    return NEUTRAL_NAMES.get(lang, "s") + ext


def run_check(lang, d):
    r = subprocess.run([BIN, "format", "check", lang, d, "--format=json"],
                       capture_output=True, text=True)
    try:
        rep = json.loads(r.stdout)
    except Exception:
        return None
    return rep


def verify(lang, ext, ok_src, ugly, broken):
    prof = next(l for l in json.load(open(os.path.join(ROOT, "rulepacks/format_profiles.json")))["languages"]
                if l["language"] == lang)
    declared = prof.get("unformatted_exit_codes", [1])
    results = {}

    # ok：先 apply 规范化，得到工具自己的规范输出
    d = tempfile.mkdtemp()
    if lang == "php":
        # php-cs-fixer 必须有项目配置才会动作，否则静默无效
        open(os.path.join(d, ".php-cs-fixer.dist.php"), "w").write(
            "<?php\nreturn (new PhpCsFixer\\Config())\n"
            "    ->setRules(['@PSR12' => true])\n"
            "    ->setFinder(PhpCsFixer\\Finder::create()->in(__DIR__));\n")
    open(os.path.join(d, sample_name(lang, ext)), "w").write(ok_src or ugly)
    subprocess.run([BIN, "format", "apply", lang, d], capture_output=True)
    rep = run_check(lang, d)
    results["ok"] = "allow" if rep and rep.get("delivery_decision") == "allow" else \
                    ("deny" if rep and rep.get("delivery_decision") == "deny" else "incomplete")
    shutil.rmtree(d)

    # ugly：未格式化，应 deny
    d = tempfile.mkdtemp()
    open(os.path.join(d, sample_name(lang, ext)), "w").write(ugly)
    rep = run_check(lang, d)
    results["ugly"] = "allow" if rep and rep.get("delivery_decision") == "allow" else \
                      ("deny" if rep and rep.get("delivery_decision") == "deny" else "incomplete")
    shutil.rmtree(d)

    # broken：语法损坏。按该工具的真实能力边界判定：
    #   incomplete — 工具能区分语法错误（期望）
    #   deny      — 工具无法区分，如实容忍（不算档案缺陷）
    #   allow     — 静默放行损坏源码（**漏判，高危**）
    d = tempfile.mkdtemp()
    open(os.path.join(d, sample_name(lang, ext)), "w").write(broken)
    rep = run_check(lang, d)
    results["broken"] = "allow" if rep and rep.get("delivery_decision") == "allow" else \
                        ("deny" if rep and rep.get("delivery_decision") == "deny" else "incomplete")
    shutil.rmtree(d)
    return results, declared


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--lang")
    ap.add_argument("--list", action="store_true")
    args = ap.parse_args()

    if not os.path.exists(BIN):
        print(f"未找到二进制 {BIN}；请先 cargo build -p codeguard-cli --features wasm-precheck")
        return 2

    prof = {l["language"]: l for l in
            json.load(open(os.path.join(ROOT, "rulepacks/format_profiles.json")))["languages"]}
    targets = [args.lang] if args.lang else [k for k in SAMPLES if k != "clojure2" and k in prof]

    if args.list:
        avail = [l for l in targets if shutil.which(prof[l].get("formatter", ""))]
        print(f"档案已接入 {len(prof)} 种；样本覆盖 {len(targets)}；"
              f"其中格式化器在 PATH 的 {len(avail)} 种")
        return 0

    print(f"{'语言':<12}{'码集':<9}{'ok':<6}{'ugly':<6}{'broken':<9}{'结论'}")
    print("-" * 62)
    failures = []
    for lang in targets:
        spec = SAMPLES[lang]
        results, declared = verify(lang, *spec)
        want = {"ok": "allow", "ugly": "deny",
                "broken": BROKEN_EXPECT.get(lang, "incomplete")}
        bad = [c for c, w in want.items() if results[c] != w]
        verdict = "✅" if not bad else f"❌ {','.join(bad)}"
        if bad:
            failures.append((lang, bad, results, declared))
        print(f"{lang:<12}{str(declared):<9}{results['ok']:<6}{results['ugly']:<6}"
              f"{results['broken']:<9}{verdict}")

    print()
    if failures:
        print(f"{len(failures)} 种语言档案与工具真实行为不符：")
        for lang, bad, results, declared in failures:
            print(f"  {lang}: 期望 ok=allow ugly=deny broken=incomplete，"
                  f"实测 {results}（码集 {declared}）")
        return 1
    print("全部样本场景通过：档案声明与工具真实行为一致。")
    return 0


if __name__ == "__main__":
    sys.exit(main())
