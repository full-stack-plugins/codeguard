"""开发验收：真实 Kotlin 任务、聚合与 Hook 协议；不参与 Rust 产品运行。"""
import copy
import json
import unittest
from pathlib import Path
from jsonschema import Draft202012Validator
from referencing import Registry, Resource
ROOT=Path(__file__).resolve().parents[1]
class KotlinNativeTaskSchemaTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.schemas={p.name:json.loads(p.read_text()) for p in (ROOT/"schemas").glob("*.schema.json")}
        cls.registry=Registry().with_resources([(n,Resource.from_contents(s)) for n,s in cls.schemas.items()]+[(s["$id"],Resource.from_contents(s)) for s in cls.schemas.values() if "$id" in s])
        cls.reports={n:c["report"] for n,c in json.loads((ROOT/"tests/acceptance/evidence/kotlin-native-task-confirmation-2026-10-04.json").read_text())["reports"].items()}
    def validator(self,n):return Draft202012Validator(self.schemas[n],registry=self.registry)
    def test_actual_task_feedback(self):
        for n in ["bad","fixed","context","mixed","missing"]:
            with self.subTest(n=n):self.validator("task-verification-preview-v0.16.schema.json").validate(self.reports[n])
        for n in ["next_bad","stale","next_fixed","next_context","next_mixed","next_missing"]:
            with self.subTest(n=n):self.validator("repair-brief-preview-v0.8.schema.json").validate(self.reports[n])
        self.validator("repair-brief-preview-v0.9.schema.json").validate(self.reports["initial_next"])
        for n in ["repair_ready","mixed_hook"]:self.validator("hook-execution-feedback-v0.10.schema.json").validate(self.reports[n])
        self.validator("check-feedback-v0.41.schema.json").validate(self.reports["aggregate"])
    def test_old_consumers_do_not_accept_new_protocols(self):
        for n,s in [("bad","task-verification-preview-v0.15.schema.json"),("next_bad","repair-brief-preview-v0.6.schema.json"),("initial_next","repair-brief-preview-v0.7.schema.json"),("repair_ready","hook-execution-feedback-v0.9.schema.json"),("aggregate","check-feedback-v0.40.schema.json")]:self.assertFalse(self.validator(s).is_valid(self.reports[n]),n)
    def test_cannot_forge_native_language_rule_or_tool_version(self):
        original=self.reports["bad"]["native_scan"];v=self.validator("syntax-task-recheck-v0.5.schema.json")
        for section,key,value in [("target","language","swift"),("native","version","Apple Swift 6.4"),("native","tool_identity_scope","full_toolchain")]:
            r=copy.deepcopy(original);r[section][key]=value;self.assertFalse(v.is_valid(r))
        r=copy.deepcopy(original);r["native"]["diagnostics"][0]["rule_id"]="kotlin.context.UNRESOLVED_REFERENCE";self.assertFalse(v.is_valid(r))
    def test_stale_and_clean_briefs_cannot_retain_error_positions(self):
        for n in ["stale","next_fixed","next_context","next_missing"]:
            r=copy.deepcopy(self.reports[n]);r["repair_brief"]["native_diagnostic_positions"]=self.reports["next_bad"]["repair_brief"]["native_diagnostic_positions"]
            self.assertFalse(self.validator("repair-brief-preview-v0.8.schema.json").is_valid(r),n)
    def test_hook_context_cannot_be_promoted_to_syntax(self):
        r=copy.deepcopy(self.reports["mixed_hook"]);r["local_feedback"]["native_diagnostic_positions"]=r["local_feedback"]["native_context_diagnostics"]
        self.assertFalse(self.validator("hook-execution-feedback-v0.10.schema.json").is_valid(r))
        for key,value in [("authority","trusted"),("delivery_decision","allow")]:
            r=copy.deepcopy(self.reports["repair_ready"]);r["local_feedback"][key]=value;self.assertFalse(self.validator("hook-execution-feedback-v0.10.schema.json").is_valid(r))
if __name__=="__main__":unittest.main()
