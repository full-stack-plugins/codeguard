"""开发期校验 Zig 原生首次 SDK 输出，签名夹具不代表默认宿主授权。"""
import copy
import json
from pathlib import Path
import unittest
from zig_aggregate_feedback_schema import ROOT, validator

def artifact(kind):
    return json.loads((ROOT / f'tests/acceptance/evidence/zig016-native-first-resolution-2026-10-05-{kind}.json').read_text())

class ZigNativeResolution(unittest.TestCase):
    def test_actual_protocols_and_origin(self):
        for kind, version in [('policy','1.4'),('evidence','0.5'),('receipt','0.1')]:
            validator(f'task-resolution-{kind}-v{version}.schema.json').validate(artifact(kind))
        policy,evidence,receipt=[artifact(kind) for kind in ['policy','evidence','receipt']]
        self.assertEqual(policy['identity'],receipt['identity'])
        self.assertEqual(evidence['identity'],receipt['identity'])
        self.assertIsNone(policy['grammar_sha256'])
        self.assertIsNone(evidence['grammar_sha256'])
        self.assertEqual(evidence['original_native']['status'],'diagnostics_observed')
        self.assertEqual(evidence['current_native']['status'],'completed')
        self.assertNotEqual(evidence['original_source_sha256'],evidence['current_source_sha256'])
        self.assertEqual(receipt['delivery_decision'],'not_evaluated')
        self.assertFalse(validator('task-resolution-policy-v1.0.schema.json').is_valid(policy))
        self.assertFalse(validator('task-resolution-evidence-v0.1.schema.json').is_valid(evidence))

    def test_wrong_origin_self_approval_and_nonempty_clean_are_rejected(self):
        for kind,version in [('policy','1.4'),('evidence','0.5')]:
            for field,value in [('grammar_sha256','a'*64),('approved',True)]:
                wrong=copy.deepcopy(artifact(kind));wrong[field]=value
                self.assertFalse(validator(f'task-resolution-{kind}-v{version}.schema.json').is_valid(wrong))
        evidence=artifact('evidence')
        for field,value in [('diagnostics',evidence['original_native']['diagnostics']),('version','OTP 28'),('reason','unknown')]:
            wrong=copy.deepcopy(evidence);wrong['current_native'][field]=value
            self.assertFalse(validator('task-resolution-evidence-v0.5.schema.json').is_valid(wrong))

if __name__=='__main__':
    unittest.main()
