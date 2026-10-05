"""开发验收：真实 Kotlin 单文件反馈与禁止错误晋级，不参与产品运行。"""
import copy
import json
import unittest
from pathlib import Path
from jsonschema import Draft202012Validator
from referencing import Registry, Resource
ROOT = Path(__file__).resolve().parents[1]
class KotlinNativeFeedbackSchemaTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        schemas = {p.name: json.loads(p.read_text()) for p in (ROOT / "schemas").glob("*.schema.json")}
        registry = Registry().with_resources([(n, Resource.from_contents(s)) for n,s in schemas.items()] + [(s["$id"], Resource.from_contents(s)) for s in schemas.values() if "$id" in s])
        cls.validator = Draft202012Validator(schemas["kotlin-lint-feedback-v0.1.schema.json"], registry=registry)
        cls.cases = json.loads((ROOT / "tests/acceptance/evidence/kotlin-native-single-file-2026-10-04.json").read_text())["cases"]
    def test_actual_reports(self):
        for case in self.cases:
            with self.subTest(case=case["case"]): self.validator.validate(case["report"])
    def test_cannot_promote_delivery_or_scope(self):
        for field,value in [("authority","trusted"),("delivery_decision","allow"),("coverage_proven",True),("exit_code",0),("scope","full_project")]:
            report=copy.deepcopy(self.cases[0]["report"]);report[field]=value
            self.assertFalse(self.validator.is_valid(report),field)
    def test_inconclusive_or_hidden_errors_require_native_confirmation(self):
        for case in self.cases:
            if case["report"]["setup"]["native_confirmation_required"]:
                report=copy.deepcopy(case["report"]);report["setup"]["native_confirmation_required"]=False
                self.assertFalse(self.validator.is_valid(report),case["case"])
    def test_syntax_cannot_borrow_context_rules(self):
        report=copy.deepcopy(self.cases[0]["report"])
        report["native"]["diagnostics"][0]["rule_id"]="kotlin.context.UNRESOLVED_REFERENCE"
        self.assertFalse(self.validator.is_valid(report))
if __name__ == "__main__": unittest.main()
