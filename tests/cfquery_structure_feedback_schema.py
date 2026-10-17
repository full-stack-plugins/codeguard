"""开发期捕获报告的正反例验收，不参与 Rust 产品运行时。"""
import copy
import hashlib
import json
import unittest
from zig_aggregate_feedback_schema import ROOT, validator


class CfqueryFeedback(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.records = json.loads((ROOT / 'tests/acceptance/evidence/cfquery-structure-2026-10-06.json').read_text())['records']

    def test_real_public_protocols(self):
        for key, name in [('accepted', 'grammar-probe-v0.1.schema.json'),
                          ('distinct', 'grammar-probe-v0.4.schema.json'),
                          ('first', 'hook-execution-feedback-v0.23.schema.json'),
                          ('repeat', 'hook-execution-feedback-v0.23.schema.json'),
                          ('check', 'check-feedback-v0.53.schema.json'),
                          ('confirmation', 'syntax-confirmation-observation-v0.11.schema.json')]:
            validator(name).validate(self.records[key])
            forged = copy.deepcopy(self.records[key])
            forged['delivery_decision'] = 'allow'
            self.assertFalse(validator(name).is_valid(forged), key)

    def test_original_file_and_fragment_identity_and_task(self):
        row = self.records['confirmation']['observations'][0]
        source = '<!--- 注释 --->\n<cfquery name="q">\nSELECT DISTINCT FROM users\n</cfquery>\n'.encode()
        self.assertEqual(row['source_sha256'], hashlib.sha256(source).hexdigest())
        self.assertEqual(row['fragment_source_sha256'], hashlib.sha256(b'\nSELECT DISTINCT FROM users\n').hexdigest())
        fact = row['structural_observations'][0]
        self.assertEqual(source[fact['start_byte']:fact['end_byte']], b'SELECT DISTINCT FROM')
        self.assertEqual(fact['start_row'], 2)
        self.assertEqual(row['recoveries'], [])
        self.assertFalse(row['grammar_qualified'])
        task = self.records['confirmation']['blocker_id']
        for label in ['first', 'repeat']:
            self.assertEqual(self.records[label]['local_feedback']['syntax_tasks']['tasks'][0]['task_id'], task)
        self.assertEqual(self.records['check']['syntax_tasks']['tasks'][0]['task_id'], task)

    def test_malformed_evidence_is_rejected(self):
        row = self.records['confirmation']['observations'][0]
        for mutate in [lambda r: r.pop('fragment_source_sha256'),
                       lambda r: r.update(fragment_source_sha256='bad'),
                       lambda r: r['structural_observations'][0].update(rule_id='codeguard.python.required_suite'),
                       lambda r: r['structural_observations'][0].update(start_byte=-1),
                       lambda r: r.update(grammar_qualified=True)]:
            forged = copy.deepcopy(row)
            mutate(forged)
            self.assertFalse(validator('cfquery-candidate-row-v0.1.schema.json').is_valid(forged))

    def test_dialogue_has_actual_rule_and_no_host_source(self):
        context = self.records['conversation']['hookSpecificOutput']['additionalContext']
        self.assertIn('codeguard.cfquery.distinct_projection', context)
        self.assertNotIn('codeguard.python.required_suite', context)
        self.assertNotIn('HOST_SECRET', context)
        self.assertNotIn('despite missing SQL select list', context)
        self.assertLessEqual(len(context), 1200)
        self.assertIn('datasource', json.dumps(self.records['next']))


if __name__ == '__main__':
    unittest.main()
