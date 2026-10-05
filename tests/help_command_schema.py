"""开发期帮助schema回归；默认和WASM构建的实际报告保持分离。"""
import copy
import json
import unittest
from zig_aggregate_feedback_schema import ROOT, validator

def packet(kind):
    return json.loads((ROOT/f"tests/acceptance/evidence/command-help-2026-10-05-{kind}.json").read_text())

class HelpContract(unittest.TestCase):
    def test_both_actual_build_catalogues(self):
        for kind in ["default","wasm"]:
            data=packet(kind);validator("command-help-v0.1.schema.json").validate(data)
            ids={row["tracking_id"] for row in data["commands"] if row["tracking_id"]}
            self.assertEqual(ids,{f"C{i:02}" for i in range(1,37)})
            self.assertEqual(data["delivery_decision"],"not_evaluated")
            probe=next(row for row in data["commands"] if row["command"]=="grammar probe")
            self.assertEqual(probe["executable"],kind=="wasm")
    def test_unknown_version_and_false_execution_claim_are_rejected(self):
        data=packet("default")
        for field,value in [("schema_version","2.0.0"),("native_execution","completed"),("delivery_decision","allow"),("approved",True)]:
            wrong=copy.deepcopy(data);wrong[field]=value
            self.assertFalse(validator("command-help-v0.1.schema.json").is_valid(wrong))
        for kind in ["default","wasm"]:
            wrong=packet(kind);row=next(row for row in wrong["commands"] if row["command"]=="mcp serve");row["executable"]=True
            self.assertFalse(validator("command-help-v0.1.schema.json").is_valid(wrong))
        wrong=packet("default");row=next(row for row in wrong["commands"] if row["command"]=="mcp serve");row["support"]="implemented";row["executable"]=True
        self.assertFalse(validator("command-help-v0.1.schema.json").is_valid(wrong))
        wrong=packet("default");row=next(row for row in wrong["commands"] if row["command"]=="lint");row["languages"].append("ruby")
        self.assertFalse(validator("command-help-v0.1.schema.json").is_valid(wrong))
if __name__=="__main__":unittest.main()
