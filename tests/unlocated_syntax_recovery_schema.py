"""开发验收：复核已归档真实报告的协议边界，不参与产品运行或替代当前原生复检。"""

import copy
import json
import unittest
from pathlib import Path

from jsonschema import Draft202012Validator
from referencing import Registry, Resource

ROOT = Path(__file__).resolve().parents[1]


class UnlocatedSyntaxRecoverySchemaTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.schemas = {
            path.name: json.loads(path.read_text())
            for path in (ROOT / "schemas").glob("*.schema.json")
        }
        cls.registry = Registry().with_resources(
            [(name, Resource.from_contents(schema)) for name, schema in cls.schemas.items()]
            + [(schema["$id"], Resource.from_contents(schema))
               for schema in cls.schemas.values() if "$id" in schema]
        )
        cls.evidence = json.loads((ROOT / "tests/acceptance/evidence/"
                                   "unlocated-syntax-recovery-2026-10-04.json").read_text())

    def validator(self, name):
        schema = self.schemas[name]
        Draft202012Validator.check_schema(schema)
        return Draft202012Validator(schema, registry=self.registry)

    def test_actual_aggregate_next_accepts_full_current_brief(self):
        validator = self.validator("check-feedback-v0.38.schema.json")
        observed = self.evidence["cases"][0]
        for name in ("first_check", "repeat_check"):
            report = observed[name]
            self.assertEqual(report["next"]["schema_version"], "0.3.0")
            self.assertEqual(len(report["syntax_tasks"]["tasks"]), 3)
            validator.validate(report)
        validator.validate(self.evidence["cases"][1]["check"])

    def test_forged_nested_next_does_not_authorize_repair_or_delivery(self):
        validator = self.validator("check-feedback-v0.38.schema.json")
        actual = self.evidence["cases"][0]["first_check"]
        for field, value in (("schema_version", "999.0.0"),
                             ("authority", "trusted"),
                             ("delivery_decision", "allow")):
            forged = copy.deepcopy(actual)
            forged["next"][field] = value
            with self.subTest(field=field):
                self.assertFalse(validator.is_valid(forged))
        forged = copy.deepcopy(actual)
        del forged["next"]["repair_brief"]["evidence_ref"]
        self.assertFalse(validator.is_valid(forged))

    def test_unlocated_reports_require_incomplete_reason_and_empty_positions(self):
        validator = self.validator("syntax-confirmation-observation-v0.3.schema.json")
        originals = self.evidence["confirmation_reports"]
        self.assertGreater(len(originals), 0)
        for actual in originals:
            validator.validate(actual)
            self.assertEqual(actual["observations"][0]["recovery_count"], 0)
            self.assertEqual(actual["observations"][0]["recoveries"], [])
            for field, value in (("reason", None), ("grammar_qualified", True),
                                 ("recovery_count", 1)):
                forged = copy.deepcopy(actual)
                forged["observations"][0][field] = value
                with self.subTest(run=actual["run_id"], field=field):
                    self.assertFalse(validator.is_valid(forged))

    def test_previous_schema_rejects_new_report_without_reinterpreting_it(self):
        actual = self.evidence["cases"][0]["first_check"]
        self.assertFalse(self.validator("check-feedback-v0.37.schema.json").is_valid(actual))
        original = self.evidence["confirmation_reports"][0]
        self.assertFalse(self.validator("syntax-confirmation-observation.schema.json")
                         .is_valid(original))


if __name__ == "__main__":
    unittest.main()
