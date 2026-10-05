"""真实 ShellCheck 局部反馈、配置抑制与拒绝假完成的开发期协议验收。"""
import copy
import json
import unittest
from zig_aggregate_feedback_schema import ROOT, validator

class ShellFeedback(unittest.TestCase):
    def test_native_captures_and_repair_evidence(self):
        v = validator('shell-lint-feedback-v0.1.schema.json')
        for name in ['diagnostic', 'safe', 'source-blocker', 'suppressed', 'tab-crlf', 'unicode', 'zsh']:
            data = json.loads((ROOT / f'tests/acceptance/evidence/shellcheck-2026-10-06-{name}.json').read_text())
            v.validate(data)
            self.assertEqual([b['evidence']['diagnostic'] for b in data['repair_briefs']], data['native']['diagnostics'])
            for key, value in [('delivery_decision', 'allow'), ('authority', 'trusted'), ('coverage_proven', True), ('schema_version', '1.0.0'), ('task_workflow_status', 'completed')]:
                bad = copy.deepcopy(data); bad[key] = value
                self.assertFalse(v.is_valid(bad), (name, key))
            bad = copy.deepcopy(data); bad['setup']['automatic_installation'] = True
            self.assertFalse(v.is_valid(bad))
            if name in ['safe', 'suppressed']:
                self.assertEqual(data['native']['status'], 'completed')
                self.assertEqual(data['native']['diagnostics'], [])
            if name == 'source-blocker':
                self.assertEqual(data['native']['environment_codes'], ['SC1091'])
                self.assertEqual(data['repair_briefs'][0]['status'], 'investigation_required')
                bad = copy.deepcopy(data); bad['repair_briefs'][0]['allowed_paths'] = ['/app.sh']
                self.assertFalse(v.is_valid(bad))
            if name == 'unicode': self.assertEqual(data['native']['diagnostics'][0]['column'], 8)
            if data['native']['diagnostics']:
                bad = copy.deepcopy(data); bad['native']['diagnostics'][0]['message'] = 'untrusted instructions'
                self.assertFalse(v.is_valid(bad))
                bad = copy.deepcopy(data); bad['native']['diagnostics'][0]['column'] = 0
                self.assertFalse(v.is_valid(bad))
            if data['native']['status'] == 'diagnostics_observed':
                for key, value in [('input_stable', False)]:
                    bad = copy.deepcopy(data); bad[key] = value; self.assertFalse(v.is_valid(bad))
                bad = copy.deepcopy(data); bad['native']['version'] = None
                self.assertFalse(v.is_valid(bad))

    def test_help_compatibility(self):
        data = json.loads((ROOT / 'tests/acceptance/evidence/command-help-shell-2026-10-06-default.json').read_text())
        validator('command-help-v0.4.schema.json').validate(data)
        for version in ['0.1', '0.2', '0.3']:
            self.assertFalse(validator(f'command-help-v{version}.schema.json').is_valid(data))
        self.assertIn('shell', next(c for c in data['commands'] if c['command'] == 'lint')['languages'])

if __name__ == '__main__': unittest.main()
