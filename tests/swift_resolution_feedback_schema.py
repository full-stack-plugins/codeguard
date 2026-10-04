"""开发期校验实际 Swift SDK 输出；结构有效不证明真实宿主信任来源。"""
import copy
import json
from pathlib import Path
import unittest
from jsonschema import Draft202012Validator
ROOT = Path(__file__).resolve().parents[1]
def load(path):
    return json.loads((ROOT / path).read_text())
def artifact(kind):
    return load(f"tests/acceptance/evidence/swift64-native-resolution-2026-10-05-{kind}.json")
def validator(version, kind):
    return Draft202012Validator(load(f"schemas/task-resolution-{kind}-v{version}.schema.json"))
class SwiftResolutionFeedback(unittest.TestCase):
    def test_real_policy_evidence_and_receipt_match_protocols(self):
        for kind, version in [("policy", "1.2"), ("evidence", "0.3"), ("receipt", "0.1")]:
            validator(version, kind).validate(artifact(kind))
        policy, evidence, receipt = [artifact(k) for k in ["policy", "evidence", "receipt"]]
        self.assertEqual(policy["identity"], receipt["identity"])
        self.assertEqual(evidence["identity"], receipt["identity"])
        self.assertIsNone(evidence["grammar_sha256"])
        self.assertEqual(evidence["original_native"]["status"], "diagnostics_observed")
        self.assertEqual(evidence["current_native"]["status"], "completed")
        self.assertNotEqual(evidence["original_source_sha256"], evidence["current_source_sha256"])
        self.assertEqual(receipt["delivery_decision"], "not_evaluated")
    def test_swift_outputs_are_not_old_language_protocols(self):
        for kind, versions in [("policy", ["1.0", "1.1"]), ("evidence", ["0.1", "0.2"])]:
            for version in versions:
                self.assertFalse(validator(version, kind).is_valid(artifact(kind)))
    def test_completed_observation_cannot_contain_diagnostics_or_wrong_tool_version(self):
        original = artifact("evidence")
        for field, value in [("diagnostics", original["original_native"]["diagnostics"]), ("version", "OTP 28"), ("reason", "erlang_native_forms_no_diagnostics")]:
            bad = copy.deepcopy(original)
            bad["current_native"][field] = value
            self.assertFalse(validator("0.3", "evidence").is_valid(bad), field)
    def test_self_approved_or_project_allow_fields_are_rejected(self):
        for kind, version in [("policy", "1.2"), ("evidence", "0.3"), ("receipt", "0.1")]:
            bad = artifact(kind)
            bad["approved"] = True
            self.assertFalse(validator(version, kind).is_valid(bad))
if __name__ == "__main__":
    unittest.main()
