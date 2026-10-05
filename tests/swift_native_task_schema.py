"""开发验收：核对归档真实 Swift 反馈及跨消费者协议，不参与 Rust 产品运行。"""
import copy
import json
import unittest
from pathlib import Path
from jsonschema import Draft202012Validator
from referencing import Registry, Resource

ROOT = Path(__file__).resolve().parents[1]

class SwiftNativeTaskSchemaTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.schemas = {p.name: json.loads(p.read_text()) for p in (ROOT / "schemas").glob("*.schema.json")}
        cls.registry = Registry().with_resources(
            [(n, Resource.from_contents(s)) for n, s in cls.schemas.items()]
            + [(s["$id"], Resource.from_contents(s)) for s in cls.schemas.values() if "$id" in s])
        cls.case = json.loads((ROOT / "tests/acceptance/evidence/swift-native-task-confirmation-2026-10-04.json").read_text())["cases"][0]

    def validator(self, name):
        return Draft202012Validator(self.schemas[name], registry=self.registry)

    def test_actual_native_feedback_and_summaries(self):
        for name in ("bad", "fixed", "missing", "unicode_bad"):
            self.validator("task-verification-preview-v0.15.schema.json").validate(self.case[name])
        for name in ("next_bad", "next_fixed", "stale"):
            self.validator("repair-brief-preview-v0.6.schema.json").validate(self.case[name])
        self.validator("hook-execution-feedback-v0.9.schema.json").validate(self.case["repair_ready_hook"])
        self.validator("check-feedback-v0.39.schema.json").validate(self.case["aggregate"])

    def test_previous_consumers_reject_new_protocols(self):
        for name, schema in (("bad", "task-verification-preview-v0.14.schema.json"),
                             ("next_bad", "repair-brief-preview-v0.5.schema.json"),
                             ("aggregate", "check-feedback-v0.38.schema.json"),
                             ("repair_ready_hook", "hook-execution-feedback-v0.8.schema.json")):
            self.assertFalse(self.validator(schema).is_valid(self.case[name]), name)

    def test_native_language_rule_and_reason_are_bound(self):
        validator = self.validator("syntax-task-recheck-v0.4.schema.json")
        actual = self.case["bad"]["native_scan"]
        for field, value in (("language", "erlang"),):
            forged = copy.deepcopy(actual)
            forged["target"][field] = value
            self.assertFalse(validator.is_valid(forged))
        for field, value in (("version", "OTP 28"), ("reason", "swift_native_parse_no_diagnostics")):
            forged = copy.deepcopy(actual)
            forged["native"][field] = value
            self.assertFalse(validator.is_valid(forged))
        forged = copy.deepcopy(actual)
        forged["native"]["diagnostics"][0]["rule_id"] = "erlang.syntax.error"
        self.assertFalse(validator.is_valid(forged))

    def test_stale_or_clean_brief_cannot_keep_error_positions(self):
        validator = self.validator("repair-brief-preview-v0.6.schema.json")
        for name in ("next_fixed", "stale"):
            forged = copy.deepcopy(self.case[name])
            forged["repair_brief"]["native_diagnostic_positions"] = self.case["next_bad"]["repair_brief"]["native_diagnostic_positions"]
            self.assertFalse(validator.is_valid(forged), name)
        forged = copy.deepcopy(self.case["next_bad"])
        forged["repair_brief"]["native_column_unit"] = "unicode_scalar"
        self.assertFalse(validator.is_valid(forged))

    def test_nested_aggregate_and_hook_cannot_promote_authority(self):
        validator = self.validator("check-feedback-v0.39.schema.json")
        for field, value in (("authority", "trusted"), ("delivery_decision", "allow"), ("schema_version", "9.0.0")):
            forged = copy.deepcopy(self.case["aggregate"])
            forged["next"][field] = value
            self.assertFalse(validator.is_valid(forged), field)
        forged = copy.deepcopy(self.case["repair_ready_hook"])
        forged["local_feedback"]["native_scan"] = self.case["bad"]["native_scan"]
        self.assertFalse(self.validator("hook-execution-feedback-v0.9.schema.json").is_valid(forged))

if __name__ == "__main__":
    unittest.main()
