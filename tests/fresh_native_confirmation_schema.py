"""开发验收：核对首次确认指引的实际 CLI 报告，不参与产品检测运行。"""
import copy
import json
import unittest
from pathlib import Path

from jsonschema import Draft202012Validator
from referencing import Registry, Resource

ROOT = Path(__file__).resolve().parents[1]


class FreshNativeConfirmationSchemaTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.schemas = {p.name: json.loads(p.read_text()) for p in (ROOT / "schemas").glob("*.schema.json")}
        cls.registry = Registry().with_resources(
            [(n, Resource.from_contents(s)) for n, s in cls.schemas.items()]
            + [(s["$id"], Resource.from_contents(s)) for s in cls.schemas.values() if "$id" in s])
        cls.evidence = json.loads((ROOT / "tests/acceptance/evidence/"
                                  "fresh-native-confirmation-guidance-2026-10-04.json").read_text())

    def validator(self, name):
        return Draft202012Validator(self.schemas[name], registry=self.registry)

    def test_actual_reports_keep_preparation_separate_from_native_execution(self):
        self.validator("repair-brief-preview-v0.7.schema.json").validate(self.evidence["next"])
        body = self.evidence["task_show"]["task"]
        self.validator("task-show-preview.schema.json").validate(self.evidence["task_show"])
        schema = self.schemas["repair-brief-preview-v0.7.schema.json"]["$defs"]["syntax_preparation_brief"]
        Draft202012Validator(schema, registry=self.registry).validate(body)
        self.validator("check-feedback-v0.40.schema.json").validate(self.evidence["aggregate"])
        self.assertEqual(body["tool_readiness"], "not_evaluated")
        self.assertEqual(self.evidence["task_show"]["next_actions"], [body["recheck_argv"]])
        self.assertNotIn("native_confirmation_ref", body)
        self.assertFalse(self.evidence["native_recheck_executed"])
        self.assertEqual(self.evidence["finding_state"], "open")

    def test_historical_consumers_reject_new_preparation_contract(self):
        for name in ("repair-brief-preview.schema.json", "repair-brief-preview-v0.4.schema.json",
                     "repair-brief-preview-v0.5.schema.json", "repair-brief-preview-v0.6.schema.json"):
            self.assertFalse(self.validator(name).is_valid(self.evidence["next"]), name)
        self.assertFalse(self.validator("check-feedback-v0.39.schema.json").is_valid(self.evidence["aggregate"]))
        red = json.loads((ROOT / "tests/acceptance/evidence/"
                          "fresh-native-confirmation-guidance-schema-red-2026-10-04.json").read_text())
        self.assertFalse(self.validator("repair-brief-preview.schema.json").is_valid(red["next"]))

    def test_language_version_parameter_and_readiness_cannot_be_forged(self):
        validator = self.validator("repair-brief-preview-v0.7.schema.json")
        actual = self.evidence["next"]
        changes = [("language", "swift"), ("supported_tool_version", "0.17.0"),
                   ("tool_option", "--erl-tool")]
        for field, value in changes:
            forged = copy.deepcopy(actual)
            forged["repair_brief"]["native_adapter"][field] = value
            self.assertFalse(validator.is_valid(forged), field)
        for field, value in [("tool_readiness", "ready"), ("native_confirmation_status", "completed"),
                             ("authority", "trusted"), ("delivery_decision", "allow")]:
            forged = copy.deepcopy(actual)
            forged["repair_brief"][field] = value
            self.assertFalse(validator.is_valid(forged), field)
        for index, value in [(7, "--swift-tool"), (8, "/unverified/path/zig")]:
            forged = copy.deepcopy(actual)
            forged["repair_brief"]["recheck_argv"][index] = value
            self.assertFalse(validator.is_valid(forged), index)

    def test_aggregate_retains_closed_nested_schema(self):
        validator = self.validator("check-feedback-v0.40.schema.json")
        for field, value in [("schema_version", "999.0.0"), ("authority", "trusted"),
                             ("delivery_decision", "allow")]:
            forged = copy.deepcopy(self.evidence["aggregate"])
            forged["next"][field] = value
            self.assertFalse(validator.is_valid(forged), field)


if __name__ == "__main__":
    unittest.main()
