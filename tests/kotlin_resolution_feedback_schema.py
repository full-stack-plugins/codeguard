"""实际 Kotlin 原生输出的开发期协议验证；夹具批准不代表市场宿主。"""
import copy
import json
from pathlib import Path
import unittest
from jsonschema import Draft202012Validator
ROOT = Path(__file__).resolve().parents[1]
def load(path):
    return json.loads((ROOT/path).read_text())
def packet(label):
    return load(f"tests/acceptance/evidence/kotlin2410-resolution-2026-10-05-{label}.json")
def validator(kind, version):
    return Draft202012Validator(load(f"schemas/task-resolution-{kind}-v{version}.schema.json"))
class KotlinResolutionFeedback(unittest.TestCase):
    def test_actual_packets_use_separate_policy_evidence_and_shared_receipt(self):
        for label in ["fixed", "context", "mixed"]:
            p = packet(label)
            for kind, version in [("policy", "1.3"), ("evidence", "0.4"), ("receipt", "0.1")]:
                validator(kind, version).validate(p[kind])
            for kind in ["policy", "evidence"]:
                self.assertEqual(p[kind]["identity"], p["receipt"]["identity"])
                self.assertIsNone(p[kind]["grammar_sha256"])
            self.assertEqual(p["evidence"]["policy_sha256"], p["receipt"]["policy_sha256"])
            self.assertEqual(p["receipt"]["delivery_decision"], "not_evaluated")
            self.assertFalse(p["installed_marketplace_host"])
    def test_context_cannot_close_but_mixed_syntax_reopens_the_same_task(self):
        fixed, context, mixed = [packet(k) for k in ["fixed", "context", "mixed"]]
        self.assertEqual(fixed["receipt"]["state"], "resolved")
        self.assertEqual(context["receipt"]["state"], "verification_required")
        self.assertEqual(context["receipt"]["outcome"], "native_incomplete")
        self.assertEqual(mixed["receipt"]["state"], "open")
        self.assertEqual(mixed["receipt"]["outcome"], "still_present")
        self.assertEqual(context["evidence"]["current_native"]["diagnostics"], [])
        self.assertTrue(context["evidence"]["current_native"]["context_diagnostics"])
        self.assertTrue(mixed["evidence"]["current_native"]["diagnostics"])
        self.assertTrue(mixed["evidence"]["current_native"]["context_diagnostics"])
        self.assertTrue(any(e["event"]["kind"]["event"] == "reopened" for e in mixed["events"]))
        self.assertEqual(fixed["receipt"]["identity"], mixed["receipt"]["identity"])
    def test_incomplete_context_is_not_clean_and_columns_are_language_specific(self):
        for label in ["context", "mixed"]:
            bad = packet(label)["evidence"]
            bad["outcome"] = "code_fixed"
            self.assertFalse(validator("evidence", "0.4").is_valid(bad))
        bad = packet("fixed")["evidence"]
        row = bad["original_native"]["diagnostics"][0]
        row["column"] = row.pop("column_byte")
        self.assertFalse(validator("evidence", "0.4").is_valid(bad))
    def test_prior_language_protocols_and_self_approval_fields_are_rejected(self):
        for kind, versions in [("policy", ["1.0", "1.1", "1.2"]), ("evidence", ["0.1", "0.2", "0.3"])]:
            for version in versions:
                self.assertFalse(validator(kind, version).is_valid(packet("fixed")[kind]))
        for kind, version in [("policy", "1.3"), ("evidence", "0.4"), ("receipt", "0.1")]:
            bad = packet("fixed")[kind]
            bad["approved"] = True
            self.assertFalse(validator(kind, version).is_valid(bad))
if __name__ == "__main__":
    unittest.main()
