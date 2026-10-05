"""Ruby编辑与修复Hook的封闭协议回归；不以宿主重放证明真实安装。"""
import copy
import json
import unittest
from zig_aggregate_feedback_schema import ROOT, validator

class RubyHookFeedback(unittest.TestCase):
    def test_archived_feedback_and_line_only_protocol(self):
        paths=sorted((ROOT/'tests/acceptance/evidence').glob('ruby-hook-2026-10-05-*.json'))
        self.assertGreaterEqual(len(paths),6)
        for path in paths:
            data=json.loads(path.read_text())
            name='hook-execution-feedback-v0.20.schema.json' if data['schema_version']=='0.20.0' else 'hook-execution-feedback-v0.19.schema.json'
            v=validator(name);v.validate(data)
            for key,value in [('schema_version','99.0.0'),('delivery_decision','allow'),('host_blocking_verified',True),('approved',True)]:
                bad=copy.deepcopy(data);bad[key]=value;self.assertFalse(v.is_valid(bad),path.name)
            feedback=data['local_feedback']
            if data['schema_version']=='0.20.0':
                if feedback['native_diagnostic_positions']:
                    bad=copy.deepcopy(data);bad['local_feedback']['native_diagnostic_positions'][0]['column']=1
                    self.assertFalse(v.is_valid(bad))
                bad=copy.deepcopy(data);bad['local_feedback']['native_column_unit']='utf8_byte';self.assertFalse(v.is_valid(bad))
            else:
                for row in feedback['ruby_lint']['files']:
                    if row['native']['diagnostics']:
                        bad=copy.deepcopy(data);bad['local_feedback']['ruby_lint']['files'][0]['native']['diagnostics'][0]['column']=1
                        self.assertFalse(v.is_valid(bad))
                self.assertFalse(validator('hook-execution-feedback-v0.18.schema.json').is_valid(data))
                bad=copy.deepcopy(data);bad['local_feedback']['syntax_candidates']['status']='completed';self.assertFalse(v.is_valid(bad))

    def test_project_version_reports_preserve_environment_and_line_only_evidence(self):
        for label in ('mismatch', 'matching'):
            path=ROOT/'tests/acceptance/evidence'/f'ruby-project-version-2026-10-05-{label}.json'
            data=json.loads(path.read_text())
            v=validator('check-feedback-v0.51.schema.json');v.validate(data)
            native=data['native_results']['ruby_lint']['files'][0]['native']
            if label=='mismatch':
                self.assertEqual(native['status'],'incomplete')
                self.assertEqual(native['reason'],'ruby_project_version_mismatch')
                self.assertEqual(native['diagnostics'],[])
            else:
                self.assertEqual(native['status'],'diagnostics_observed')
                self.assertGreater(len(native['diagnostics']),0)
                self.assertTrue(all('column' not in row for row in native['diagnostics']))
            bad=copy.deepcopy(data);bad['delivery_decision']='allow';self.assertFalse(v.is_valid(bad))

    def test_installed_package_reports_and_host_shape(self):
        reports=json.loads((ROOT/'tests/acceptance/evidence/ruby-npm-repair-2026-10-05.json').read_text())
        mapping={'initialized':'init-plan.schema.json','first':'hook-execution-feedback-v0.19.schema.json','recurrence':'hook-execution-feedback-v0.19.schema.json','aggregate':'check-feedback-v0.51.schema.json','lint':'ruby-lint-feedback-v0.3.schema.json','next':'repair-brief-preview-v0.15.schema.json','stale':'repair-brief-preview-v0.15.schema.json','present':'hook-execution-feedback-v0.20.schema.json','repaired':'hook-execution-feedback-v0.20.schema.json'}
        for key,name in mapping.items():
            v=validator(name);v.validate(reports[key])
            if 'delivery_decision' in reports[key]:
                bad=copy.deepcopy(reports[key]);bad['delivery_decision']='allow';self.assertFalse(v.is_valid(bad))
        context=reports['conversation']['hookSpecificOutput']['additionalContext']
        self.assertLessEqual(len(context),1200);self.assertNotIn('HOST_SECRET',context);self.assertNotIn('IGNORE_GUARDS',context)
        self.assertIn(reports['first']['local_feedback']['ruby_lint']['files'][0]['task_id'],context)
        self.assertEqual(reports['repaired']['local_feedback']['observation'],'candidate_absent_unverified_policy')

if __name__=='__main__':unittest.main()
