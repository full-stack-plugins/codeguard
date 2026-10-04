"""开发验收：投影恢复报告、不可变事实及非授权语义。"""
import copy
import json
import unittest
from pathlib import Path
from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[1]

class ProjectionFeedbackTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.evidence = json.loads((ROOT / 'tests/acceptance/evidence/task-projection-recovery-2026-10-05.json').read_text())
        cls.new = Draft202012Validator(json.loads((ROOT / 'schemas/work-sync-preview-v0.3.schema.json').read_text()))
        cls.old = Draft202012Validator(json.loads((ROOT / 'schemas/work-sync-preview.schema.json').read_text()))

    def test_actual_protocol_and_old_consumer(self):
        r = self.evidence['reports']['restore']['report']
        self.new.validate(r)
        self.assertFalse(self.old.is_valid(r))
        self.old.validate(self.evidence['reports']['repeat']['report'])

    def test_recovery_does_not_change_facts_or_close_tasks(self):
        self.assertEqual(self.evidence['original_record_sha256'], self.evidence['recovered_record_sha256'])
        self.assertEqual(self.evidence['reports']['show_after']['report']['task']['task_id'], self.evidence['task_id'])
        self.assertEqual(self.evidence['final_state'], 'open')
        self.assertFalse(self.evidence['trusted_closure'])
        self.assertFalse(self.evidence['actual_host_verified'])

    def test_recovery_requires_positive_count_and_cannot_allow_delivery(self):
        for field, value in [('restored_task_projections', 0), ('restored_task_projections', -1), ('delivery_decision', 'allow')]:
            r = copy.deepcopy(self.evidence['reports']['restore']['report'])
            r[field] = value
            self.assertFalse(self.new.is_valid(r))

    def test_readable_task_contains_all_required_components(self):
        text = self.evidence['recovered_markdown']
        for heading in ['问题证据', '规则依据', '允许修改的范围', '修复步骤', '复检命令', '历史尝试', '关闭条件']:
            self.assertIn('## ' + heading, text)
        self.assertIn('勾选、备注或删除不能关闭问题', text)
        self.assertNotIn('/private/var/', text)

if __name__ == '__main__':
    unittest.main()
