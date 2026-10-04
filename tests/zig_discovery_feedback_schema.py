"""开发期验证实际 Zig 输出；不参与 Rust 检查运行时。"""
import copy
import json
from pathlib import Path
import unittest
from jsonschema import Draft202012Validator
from referencing import Registry, Resource
ROOT=Path(__file__).resolve().parents[1]
class ZigDiscoveryFeedback(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.schemas={p.name:json.loads(p.read_text()) for p in (ROOT/'schemas').glob('*.schema.json')}
        cls.registry=Registry().with_resources([(n,Resource.from_contents(s)) for n,s in cls.schemas.items()]+[(s['$id'],Resource.from_contents(s)) for s in cls.schemas.values() if '$id' in s])
        cls.lint=json.loads((ROOT/'tests/acceptance/evidence/zig-path-lint-2026-10-05.json').read_text())
        cls.tasks=json.loads((ROOT/'tests/acceptance/evidence/zig-path-task-2026-10-05.json').read_text())
    def validator(self,name): return Draft202012Validator(self.schemas[name],registry=self.registry)
    def test_actual_lint_reports_are_current_and_native_first(self):
        for report in self.lint.values():
            self.validator('zig-lint-feedback-v0.2.schema.json').validate(report)
            self.assertTrue(report['source_current'])
            self.assertEqual(report['tool_selection']['source'],'path')
            self.assertIsNone(report['syntax_precheck'])
            self.assertFalse(self.validator('zig-lint-feedback-v0.1.schema.json').is_valid(report))
        self.assertEqual(self.lint['clean']['native']['diagnostic_count'],0)
        self.assertGreater(self.lint['broken']['native']['diagnostic_count'],0)
    def test_old_positions_cannot_be_declared_current_after_source_changes(self):
        report=copy.deepcopy(self.lint['broken']);report['source_current']=False
        self.assertFalse(self.validator('zig-lint-feedback-v0.2.schema.json').is_valid(report))
        report['native']['status']='incomplete';report['native']['reason']='zig_source_changed_during_check';report['native']['diagnostics']=[];report['native']['diagnostic_count']=0
        self.validator('zig-lint-feedback-v0.2.schema.json').validate(report)
    def test_completion_cannot_have_diagnostics_or_missing_tool_provenance(self):
        report=copy.deepcopy(self.lint['broken']);report['native']['status']='completed'
        self.assertFalse(self.validator('zig-lint-feedback-v0.2.schema.json').is_valid(report))
        report=copy.deepcopy(self.lint['clean']);report['tool_selection']={'source':'not_found','executable':None}
        self.assertFalse(self.validator('zig-lint-feedback-v0.2.schema.json').is_valid(report))
    def test_real_task_observations_preserve_identity_and_open_state(self):
        for key, report in self.tasks.items():
            if key=='fact_state': continue
            self.validator('task-verification-preview.schema.json').validate(report)
            self.assertTrue(report['event_persisted'])
            self.assertEqual(report['delivery_decision'],'not_evaluated')
        self.assertEqual(self.tasks['fact_state'],'open')
        self.assertEqual(self.tasks['explicit_broken']['task_id'],self.tasks['path_fixed']['task_id'])
        self.assertEqual(self.tasks['explicit_broken']['native_scan']['tool_path'],self.tasks['path_broken']['native_scan']['tool_path'])
        self.assertEqual(self.tasks['path_fixed']['observation'],'candidate_absent_unverified_policy')
if __name__=='__main__': unittest.main()
