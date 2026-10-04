"""开发验收：独立源码选择保留延后问题、预算及非授权语义。"""
import copy
import json
import unittest
from pathlib import Path
from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[1]

class IndependentSourceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.evidence = json.loads((ROOT / 'tests/acceptance/evidence/next-independent-work-2026-10-05.json').read_text())
        cls.schema = Draft202012Validator(json.loads((ROOT / 'schemas/repair-brief-preview-v0.1.schema.json').read_text()))

    def test_actual_query_uses_existing_closed_protocol(self):
        report = self.evidence['reports']['next_after']['report']
        self.schema.validate(report)
        self.assertEqual(report['schema_version'], '0.1.0')
        self.assertEqual(report['disposition'], 'actionable')
        self.assertEqual(report['delivery_decision'], 'not_evaluated')
        self.assertEqual(report['next_actions'][0], ['codeguard', 'task', 'show', self.evidence['deferred_task_id'], '.'])

    def test_deferred_budget_and_records_are_preserved(self):
        e = self.evidence
        self.assertNotEqual(e['selected_task_id'], e['deferred_task_id'])
        self.assertEqual(e['original_record_sha256'], e['after_query_record_sha256'])
        brief = e['reports']['deferred_show']['report']['task']
        self.assertEqual(brief['disposition'], 'needs_decision')
        self.assertEqual(brief['history']['no_progress_count'], 2)
        self.assertFalse(e['trusted_closure'])
        self.assertFalse(e['actual_installed_host'])

    def test_human_feedback_retains_both_tasks_without_reusable_token(self):
        e = self.evidence
        self.assertIn(e['selected_task_id'], e['human_next'])
        self.assertIn(e['deferred_task_id'], e['human_next'])
        self.assertIn('只读查询', e['human_next'])
        self.assertNotIn('"lease_token":', json.dumps(e))
        self.assertEqual(e['reports']['release']['exit_code'], 0)

    def test_query_cannot_upgrade_to_delivery_allow(self):
        report = copy.deepcopy(self.evidence['reports']['next_after']['report'])
        report['delivery_decision'] = 'allow'
        self.assertFalse(self.schema.is_valid(report))

if __name__ == '__main__':
    unittest.main()
