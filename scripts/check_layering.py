#!/usr/bin/env python3
"""分层门禁：阻止领域逻辑继续堆积在 codeguard-cli 这个二进制 crate。

背景：codeguard 把执行内核切成四个 crate，设计意图是
  core（契约与领域类型）← runtime（执行）← adapters（工具适配）← cli（组装）
但实际上一度出现倒挂——adapters 已有的工具诊断类型，其身份映射逻辑却住在 cli，
让 cli 膨胀到远大于 core，且无法单独测试领域规则。

本门禁不试图一次性搬完（那会让 diff 无法审阅），而是把「倒挂」变成显式债务：
cli 中仍然存在的领域形态模块被登记在 layer-baseline.json 里，数量只减不增。
新增领域模块而不同步下移，会让本检查失败并指出应该去哪个 crate。

用法：
  python3 scripts/check_layering.py            # 检查
  python3 scripts/check_layering.py --explain  # 打印每个模块的归属理由
  python3 scripts/check_layering.py --accept <name>  # 登记一个既有模块（带理由）
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

# 领域形态：这些后缀的模块在 cli 中几乎都属于"工具适配 / 报告构造 / 领域类型"，
# 而不是命令行管道。cli 只应保留参数解析、终端输出与编排。
DOMAIN_SHAPED = (
    "_probe",
    "_identity",
    "_observation",
    "_diagnostic",
    "_parsed",
    "_config",
    "_graph",
    "_state",
    "_model",
)

# 每个后缀应归属的 crate，用于给出可执行的迁移建议。
HOME_FOR = {
    "_probe": "codeguard-adapters",
    "_identity": "codeguard-adapters",
    "_diagnostic": "codeguard-adapters",
    "_parsed": "codeguard-adapters",
    "_config": "codeguard-adapters",
    "_observation": "codeguard-adapters",
    "_graph": "codeguard-core",
    "_state": "codeguard-core",
    "_model": "codeguard-core",
}

# 命令入口与管道是 cli 的正当职责，不计入债务。
CLI_ALLOWED = ("_command", "_arguments", "_preview", "_brief", "_feedback", "main.rs", "lib.rs")


def domain_modules() -> dict[str, str]:
    """返回 {模块名: 应归属 crate}，只含 cli 中本应下移的领域模块。"""
    found: dict[str, str] = {}
    for path in sorted(CLI_SRC.rglob("*.rs")):
        rel = path.relative_to(CLI_SRC).as_posix()
        if rel in CLI_ALLOWED or any(rel.endswith(a) for a in CLI_ALLOWED):
            continue
        stem = rel[:-3]
        if stem.endswith(CLI_ALLOWED[:-1]) or stem in ("main", "lib"):
            continue
        for suffix, home in HOME_FOR.items():
            if stem.endswith(suffix):
                found[rel] = home
                break
    return found


def load_baseline() -> dict:
    if not BASELINE.is_file():
        return {"version": 1, "accepted": {}}
    return json.loads(BASELINE.read_text(encoding="utf-8"))


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="codeguard crate 分层门禁")
    parser.add_argument("--explain", action="store_true", help="打印每个模块的归属理由")
    parser.add_argument("--accept", metavar="NAME", help="把一个既有模块登记为已知债务")
    parser.add_argument("--reason", metavar="TEXT", help="配合 --accept 记录理由")
    args = parser.parse_args(argv)

    current = domain_modules()
    baseline = load_baseline()
    accepted: dict = baseline.get("accepted", {})

    if args.accept:
        if args.accept not in current:
            print(f"{args.accept} 不在当前 cli 领域模块清单中，无需登记", file=sys.stderr)
            return 2
        accepted[args.accept] = args.reason or "未记录理由"
        baseline["accepted"] = accepted
        BASELINE.write_text(
            json.dumps(baseline, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        print(f"已登记 {args.accept} → {HOME_FOR.get(suffix_of(args.accept), 'codeguard-core')}")
        return 0

    if args.explain:
        for name in sorted(current):
            state = "已知债务" if name in accepted else "未登记（会被本检查拒绝）"
            print(f"{name:52} → {current[name]:20} [{state}]")
            if name in accepted:
                print(f"{'':52}   理由：{accepted[name]}")
        return 0

    # 未登记的新领域模块 = 倒挂在扩大
    undeclared = sorted(set(current) - set(accepted))

    if undeclared:
        print("分层门禁失败：以下领域形态模块新增在 codeguard-cli 中。", file=sys.stderr)
        print("它们应随工具适配或领域契约下移，而不是留在二进制 crate：\n", file=sys.stderr)
        for name in undeclared:
            print(f"  {name}  →  应归属 {current[name]}", file=sys.stderr)
        print(
            "\n处理方式：把模块移到对应 crate 并更新调用方；"
            "若确有理由暂留，用 --accept 显式登记为已知债务。",
            file=sys.stderr,
        )
        return 1

    # 已登记模块被移走了：基线应收紧，不留空壳额度
    removed = sorted(set(accepted) - set(current))
    stale = [n for n in removed if n in accepted]

    if stale:
        print("分层门禁失败：以下已登记模块已不存在，请从 layer-baseline.json 移除，", file=sys.stderr)
        print("以便额度随搬移自动收紧：\n", file=sys.stderr)
        for name in stale:
            print(f"  {name}", file=sys.stderr)
        return 1

    print(
        f"Layering OK: codeguard-cli 中已知领域债务 {len(current)} 个"
        f"（全部已登记，无新增）；core/adapters 未被 CLI 逻辑反向污染。"
    )
    return 0


def suffix_of(name: str) -> str:
    stem = name[:-3] if name.endswith(".rs") else name
    for suffix in HOME_FOR:
        if stem.endswith(suffix):
            return suffix
    return ""


if __name__ == "__main__":
    raise SystemExit(main())
