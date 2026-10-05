"""开发验收：核验真实已发布 Rust 库存报告与伪造反例；不参与产品运行。"""
import copy
import json
import os
import subprocess
import unittest
from pathlib import Path

from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[1]


class GrammarInventorySchemaTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.schema = json.loads((ROOT / "schemas/grammar-inventory-v1.1.schema.json").read_text())
        Draft202012Validator.check_schema(cls.schema)
        cls.validator = Draft202012Validator(cls.schema)
        binary = os.environ.get("CODEGUARD_INVENTORY_BIN", str(ROOT / "target/release/codeguard"))
        result = subprocess.run([binary, "grammar", "status", "--format=json"],
                                capture_output=True, check=False, timeout=10)
        if result.returncode != 3:
            raise AssertionError("real grammar inventory must retain incomplete exit 3")
        cls.report = json.loads(result.stdout)

    def test_real_published_inventory_matches_schema(self):
        self.validator.validate(self.report)

    def test_forged_authority_completion_and_asset_release_rejected(self):
        for field, value in [("schema_version", "2.0.0"), ("authority", "trusted"),
                             ("delivery_decision", "allow"), ("execution", "completed"),
                             ("released_count", 1), ("parser_capability", "qualified"), ("candidate_count", 0)]:
            report = copy.deepcopy(self.report)
            report[field] = value
            with self.subTest(field=field):
                self.assertFalse(self.validator.is_valid(report))
        report = copy.deepcopy(self.report)
        report["assets"][0]["released"] = True
        self.assertFalse(self.validator.is_valid(report))

    def test_missing_duplicate_language_and_unknown_field_rejected(self):
        variants = []
        report = copy.deepcopy(self.report)
        report["assets"].pop()
        variants.append(report)
        report = copy.deepcopy(self.report)
        report["assets"][0]["language"] = report["assets"][1]["language"]
        variants.append(report)
        report = copy.deepcopy(self.report)
        report["assets"][0]["approval"] = "self-issued"
        variants.append(report)
        for report in variants:
            self.assertFalse(self.validator.is_valid(report))

    def test_false_positive_limitations_are_bounded_and_cannot_be_removed(self):
        for value in [[], [""], ["x" * 513], ["secret\ncommand"]]:
            report = copy.deepcopy(self.report)
            report["assets"][0]["known_limitations"] = value
            with self.subTest(value=value):
                self.assertFalse(self.validator.is_valid(report))


if __name__ == "__main__":
    unittest.main()
