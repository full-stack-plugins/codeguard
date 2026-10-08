#!/usr/bin/env python3
"""合并 rulepacks/format_batches/*.json 分片为 rulepacks/format_profiles.json。

合并规则：
1. 以 legacy_languages.json 的顺序为准，输出 57 条语言档案
2. 分片档案覆盖同 id 的既有档案；分片未覆盖的语言保留现有值
3. 任何契约违例（缺字段 / 扩展名缺前导点 / argv 缺 {file} / id 不在注册表）直接失败
4. 不静默丢弃：分片里出现注册表外的 language id 时报错
"""

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
REGISTRY = ROOT / "rulepacks" / "legacy_languages.json"
BATCH_DIR = ROOT / "rulepacks" / "format_batches"
TARGET = ROOT / "rulepacks" / "format_profiles.json"

REQUIRED_FIELDS = [
    "language",
    "status",
    "formatter",
    "tool_key",
    "check_argv",
    "apply_argv",
    "config_markers",
    "extensions",
    "dialects",
    "notes",
]


def fail(message: str) -> None:
    print(f"合并失败: {message}", file=sys.stderr)
    raise SystemExit(1)


def validate(entry: dict, source: str, known_ids: set) -> None:
    language = entry.get("language")
    if not isinstance(language, str) or not language:
        fail(f"{source}: language 缺失或非字符串")
    if language not in known_ids:
        fail(f"{source}: {language} 不在 legacy_languages.json 注册表内")

    for field in REQUIRED_FIELDS:
        if field not in entry:
            fail(f"{source}: {language} 缺字段 {field}")

    status = entry["status"]
    if status not in ("integrated", "not_integrated"):
        fail(f"{source}: {language} status 非法: {status}")

    notes = entry["notes"]
    if not isinstance(notes, str) or not notes.strip():
        fail(f"{source}: {language} notes 不得为空")

    if status == "not_integrated":
        # 如实披露：不得编造工具身份
        if entry["formatter"] is not None or entry["tool_key"] is not None:
            fail(f"{source}: {language} 未接入时 formatter/tool_key 必须为 null")
        if entry["check_argv"] is not None or entry["apply_argv"] is not None:
            fail(f"{source}: {language} 未接入时 check_argv/apply_argv 必须为 null")
        return

    for field in ("formatter", "tool_key"):
        value = entry[field]
        if not isinstance(value, str) or not value.strip():
            fail(f"{source}: {language} 已接入时 {field} 不得为空")

    for field in ("check_argv", "apply_argv"):
        argv = entry[field]
        if not isinstance(argv, list) or not argv:
            fail(f"{source}: {language} 已接入时 {field} 必须是非空数组")
        if not all(isinstance(token, str) for token in argv):
            fail(f"{source}: {language} {field} 元素必须全部是字符串")
        if not any("{file}" in token for token in argv):
            fail(f"{source}: {language} {field} 必须含 {{file}} 占位")

    for ext in entry["extensions"]:
        if not isinstance(ext, str) or not ext.startswith("."):
            fail(f"{source}: {language} 扩展名 {ext!r} 缺前导点")

    for marker in entry["config_markers"]:
        if not isinstance(marker, str) or not marker or "/" in marker:
            fail(f"{source}: {language} 配置标记 {marker!r} 非法（应为纯文件名）")


def main() -> None:
    registry = json.loads(REGISTRY.read_text(encoding="utf-8"))
    ordered_ids = [lang["id"] for lang in registry["languages"]]
    known_ids = set(ordered_ids)

    current = json.loads(TARGET.read_text(encoding="utf-8"))
    merged = {row["language"]: dict(row) for row in current["languages"]}

    batch_files = sorted(BATCH_DIR.glob("*.json"))
    if not batch_files:
        fail(f"{BATCH_DIR} 下没有分片文件")

    # 收尾分片必须最后处理：它基于更晚的实测证据，覆盖先前分片的判定。
    # 不能依赖 glob 字母序（leftover < legacy，顺序恰好相反）。
    tail = [f for f in batch_files if f.stem == "leftover"]
    batch_files = [f for f in batch_files if f.stem != "leftover"] + tail

    # leftover 分片允许覆盖先前判定（它最后处理，携带更新的实测证据）。
    overrides = {
        "dockerfile", "haskell", "fsharp", "groovy", "powershell", "pascal",
        "cfml", "vbnet", "liquid", "ansible", "cobol", "arkts", "metal",
    }
    contributors: dict[str, str] = {}
    for path in batch_files:
        doc = json.loads(path.read_text(encoding="utf-8"))
        if doc.get("schema_version") != "0.1.0":
            fail(f"{path.name}: schema_version 应为 0.1.0")
        for entry in doc.get("languages", []):
            validate(entry, path.name, known_ids)
            language = entry["language"]
            if language in contributors and language not in overrides:
                fail(
                    f"{path.name}: {language} 与 {contributors[language]} 重复分片"
                )
            if language in contributors:
                # 收尾分片（leftover）显式覆盖先前判定：它基于更新的实测证据。
                print(f"  覆盖 {language}（先前来自 {contributors[language]}）")
            contributors[language] = path.name
            merged[language] = entry

    missing = [i for i in ordered_ids if i not in merged]
    if missing:
        fail(f"以下语言没有任何档案: {missing}")

    current["languages"] = [merged[i] for i in ordered_ids]
    TARGET.write_text(
        json.dumps(current, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
    )

    integrated = [row["language"] for row in current["languages"] if row["status"] == "integrated"]
    print(f"合并完成: {len(batch_files)} 个分片, {len(contributors)} 条档案")
    print(f"总计 {len(current['languages'])} 语言, 已接入 {len(integrated)}")
    print(f"已接入: {', '.join(integrated)}")


if __name__ == "__main__":
    main()
