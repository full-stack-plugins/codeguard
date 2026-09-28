#!/usr/bin/env python3
"""分层门禁：只标记「真正错位」在 codeguard-cli 中的领域模块。

权威边界不是本文档，而是 `crates/codeguard-cli/tests/crate_boundaries.rs`——
它以测试形式强制以下依赖方向：

    codeguard-core     仅依赖 serde
    codeguard-runtime  依赖 core / libc / ring（进程执行）
    codeguard-adapters 依赖 core / serde / serde_json / roxmltree / toml / sha2（纯解析与配置）
    codeguard-cli      依赖以上三者（编排）

由此得出两条常被搞反的事实：

1. 探针（*_probe）留在 cli 是正确的。它们调用 run_process_recorded，需要
   codeguard-runtime；而 adapters 被明确禁止依赖 runtime。把探针「下移」到
   adapters 会被 crate_boundaries 直接判失败。
2. 能否下移取决于依赖而非名字。一个模块若既不需要 runtime、又不依赖 cli 本地
   类型，它才是纯粹被放错位置的领域逻辑。

本门禁只检查第 2 类：它不做搬迁，只保证错位不会悄悄增加。

用法：
  python3 scripts/check_layering.py             # 检查
  python3 scripts/check_layering.py --explain   # 说明判定依据
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CLI_SRC = ROOT / "crates" / "codeguard-cli" / "src"
BASELINE = ROOT / "layer-baseline.json"

DOMAIN_SHAPED = (
    "_probe", "_identity", "_observation", "_diagnostic", "_parsed",
    "_config", "_graph", "_state", "_model", "_record", "_index",
)
CLI_ALLOWED = ("_command", "_arguments", "_preview", "_brief", "_feedback")
CLI_LOCAL = re.compile(r'\b(?:use\s+)?crate::[a-z_][a-z0-9_]*')


def audit() -> list[dict]:
    """返回真正错位的模块：命名像领域模块，且既不引用 runtime 也不引用 cli 本地类型。"""
    out: list[dict] = []
    for path in sorted(CLI_SRC.rglob("*.rs")):
        rel = path.relative_to(CLI_SRC).as_posix()
        stem = path.stem
        if stem in ("main", "lib") or any(stem.endswith(a) for a in CLI_ALLOWED):
            continue
        if not any(stem.endswith(d) for d in DOMAIN_SHAPED):
            continue
        text = path.read_text(encoding="utf-8", errors="replace")
        if "codeguard_runtime" in text or CLI_LOCAL.search(text):
            continue
        out.append({"path": rel, "lines": len(text.splitlines())})
    return out


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="codeguard crate 分层门禁")
    parser.add_argument("--explain", action="store_true", help="打印判定依据")
    args = parser.parse_args(argv)

    misplaced = audit()
    current = {i["path"] for i in misplaced}

    if args.explain:
        print("权威边界见 crates/codeguard-cli/tests/crate_boundaries.rs：")
        print("  adapters 不得依赖 runtime，因此 *_probe 留在 cli 是正确的。\n")
        if not misplaced:
            print("当前 codeguard-cli 中没有真正错位的领域模块。")
        for item in misplaced:
            print(f"  {item['path']:48} {item['lines']:5} 行  → 应下移到 adapters/core")
        return 0

    if BASELINE.is_file():
        accepted = json.loads(BASELINE.read_text(encoding="utf-8")).get("accepted", {})
        stale = set(accepted) - current
        if stale:
            print("分层门禁失败：以下登记项已不再错位，请从 layer-baseline.json 移除，"
                  "以免留下永不收紧的空额度：", file=sys.stderr)
            for name in sorted(stale):
                print(f"  {name}", file=sys.stderr)
            return 1
        new = sorted(current - set(accepted))
        if new:
            print("分层门禁失败：以下模块既不需要 codeguard_runtime、也不依赖 cli 本地类型，"
                  "属于被放错位置的领域逻辑：", file=sys.stderr)
            for name in new:
                print(f"  {name}", file=sys.stderr)
            return 1

    if misplaced:
        print("分层门禁失败：以下模块既不需要 runtime、也不依赖 cli 本地类型：", file=sys.stderr)
        for item in misplaced:
            print(f"  {item['path']}  ({item['lines']} 行)", file=sys.stderr)
        return 1

    print("Layering OK: codeguard-cli 中没有真正错位的领域模块；"
          "依赖 runtime 的探针留在 cli 符合 crate_boundaries 的既定方向。")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
