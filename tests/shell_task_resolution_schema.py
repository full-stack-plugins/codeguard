"""ShellCheck限定规则修复的开发协议验收；不参与产品运行时。"""
import copy
import json
import unittest
from zig_aggregate_feedback_schema import ROOT, validator

class ShellResolutionSchema(unittest.TestCase):
    def rows(self, mode):
        return json.loads((ROOT / f'tests/acceptance/evidence/shell-task-resolution-{mode}-2026-10-06.json').read_text())
    def validate(self, row):
        for kind, version in [('policy','1.10'), ('evidence','0.11'), ('receipt','0.2')]:
            validator(f'task-resolution-{kind}-v{version}.schema.json').validate(row[kind])
        self.assertEqual(row['receipt']['delivery_decision'], 'not_evaluated')
        self.assertEqual(row['policy']['identity'], row['evidence']['identity'])
        self.assertIsNone(row['evidence']['grammar_sha256'])
    def test_real_original_rule_and_other_rule_remain_distinct(self):
        rows=self.rows('real'); self.assertEqual(len(rows), 3)
        for row in rows: self.validate(row)
        states=[r['receipt']['state'] for r in rows]
        self.assertEqual(sorted(states), ['open','resolved','resolved'])
        closed_other=[r for r in rows if r['receipt']['state']=='resolved' and r['evidence']['current_native']['diagnostics']]
        self.assertEqual(len(closed_other),1)
        self.assertTrue(any(d['rule_id']=='SC2154' for d in closed_other[0]['evidence']['current_native']['diagnostics']))
        self.assertFalse(any(d['rule_id']=='SC2086' for d in closed_other[0]['evidence']['current_native']['diagnostics']))
    def test_controlled_suppression_is_not_a_code_fix(self):
        rows=self.rows('controlled')
        for row in rows: self.validate(row)
        suppressed=[r for r in rows if r['receipt']['outcome']=='suppression_requires_review']
        self.assertEqual(len(suppressed),1);self.assertNotEqual(suppressed[0]['receipt']['state'],'resolved')
    def test_foreign_protocol_and_incomplete_closure_are_rejected(self):
        row=self.rows('real')[0]
        for kind, version in [('policy','1.10'),('evidence','0.11')]:
            for key,value in [('schema_version','9.0.0'),('grammar_sha256','a'*64),('approved',True)]:
                bad=copy.deepcopy(row[kind]);bad[key]=value
                self.assertFalse(validator(f'task-resolution-{kind}-v{version}.schema.json').is_valid(bad))
        receipt=row['receipt']
        self.assertFalse(validator('task-resolution-receipt-v0.1.schema.json').is_valid(receipt))
        for key,value in [('schema_version','0.1.0'),('authority','local_approved'),('approved',True)]:
            bad=copy.deepcopy(receipt);bad[key]=value
            self.assertFalse(validator('task-resolution-receipt-v0.2.schema.json').is_valid(bad))
        bad=copy.deepcopy(receipt);bad['identity']['checker_id']='syntax.native_confirmation'
        self.assertFalse(validator('task-resolution-receipt-v0.2.schema.json').is_valid(bad))
        bad=copy.deepcopy(row['evidence']);bad['outcome']='code_fixed';bad['current_native']['status']='incomplete'
        self.assertFalse(validator('task-resolution-evidence-v0.11.schema.json').is_valid(bad))

if __name__=='__main__':unittest.main()
