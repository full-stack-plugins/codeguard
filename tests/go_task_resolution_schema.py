"""开发协议回归；真实Go工具与测试宿主信任夹具分开记录。"""
import copy
import json
import unittest
from zig_aggregate_feedback_schema import ROOT, validator

def artifact(kind, suffix="resolved"):
    return json.loads((ROOT / f"tests/acceptance/evidence/go1234-resolution-2026-10-05-{suffix}-{kind}.json").read_text())

class GoResolutionSchema(unittest.TestCase):
    def test_actual_real_tool_protocols(self):
        for suffix in ["resolved", "reopened"]:
            for kind, version in [("policy", "1.6"), ("evidence", "0.7"), ("receipt", "0.1")]:
                validator(f"task-resolution-{kind}-v{version}.schema.json").validate(artifact(kind, suffix))
            p,e,r=[artifact(k,suffix) for k in ["policy","evidence","receipt"]]
            self.assertEqual(p["identity"],e["identity"])
            self.assertEqual(p["identity"],r["identity"])
            self.assertEqual(r["delivery_decision"],"not_evaluated")
            for n in ["original_native","current_native"]:
                for key in ["gofmt_sha256","companion_binding_sha256","tool_sha256"]:
                    self.assertEqual(p[key], e[n][key])
            self.assertEqual(e["original_native"]["status"],"diagnostics_observed")
        self.assertEqual(artifact("evidence")["current_native"]["status"],"completed")
        self.assertEqual(artifact("receipt","reopened")["state"],"open")

    def test_old_versions_and_incomplete_companion_are_rejected(self):
        p=artifact("policy")
        for key in ["gofmt_sha256","companion_binding_sha256"]:
            wrong=copy.deepcopy(p); del wrong[key]
            self.assertFalse(validator("task-resolution-policy-v1.6.schema.json").is_valid(wrong))
        for version in ["1.0","1.3","1.5"]:
            self.assertFalse(validator(f"task-resolution-policy-v{version}.schema.json").is_valid(p))
        e=artifact("evidence")
        for field,value in [("approved",True),("grammar_sha256",None)]:
            wrong=copy.deepcopy(e);wrong[field]=value
            self.assertFalse(validator("task-resolution-evidence-v0.7.schema.json").is_valid(wrong))
        for field,value in [("version","go1.23.5"),("gofmt_sha256",None),("input_type","fragment"),("reason","unknown")]:
            wrong=copy.deepcopy(e);wrong["current_native"][field]=value
            self.assertFalse(validator("task-resolution-evidence-v0.7.schema.json").is_valid(wrong))

if __name__=="__main__": unittest.main()
