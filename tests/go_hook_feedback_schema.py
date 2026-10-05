"""Go 原生编辑与原SDK复检的实际反馈协议回归。"""
import copy
import json
import unittest
from zig_aggregate_feedback_schema import ROOT, validator

class GoHookFeedback(unittest.TestCase):
    def test_actual_edit_and_repair_feedback(self):
        prefix=ROOT/'tests/acceptance/evidence'
        for name in ['first','repeat','clean','missing','selected-failure','present','repaired']:
            data=json.loads((prefix/f'go-hook-2026-10-06-{name}.json').read_text())
            v=validator('hook-execution-feedback-v0.22.schema.json' if data['schema_version']=='0.22.0' else 'hook-execution-feedback.schema.json')
            v.validate(data)
            for key,value in [('schema_version','99.0.0'),('delivery_decision','allow'),('host_blocking_verified',True),('approved',True)]:
                bad=copy.deepcopy(data);bad[key]=value;self.assertFalse(v.is_valid(bad),name)
            if data['schema_version']=='0.22.0':
                scan=data['local_feedback']['go_syntax']
                self.assertEqual(scan['source_file_count'],1)
                self.assertEqual(scan['files'][0]['path'],'app.go')
                bad=copy.deepcopy(data);bad['local_feedback']['go_syntax']['files'][0]['native']['extra']='injected';self.assertFalse(v.is_valid(bad))
                bad=copy.deepcopy(data);bad['local_feedback']['go_syntax']['files'][0]['native']['version']='go1.24.0';self.assertFalse(v.is_valid(bad))
                if name!='missing':self.assertEqual(data['local_feedback']['syntax_candidates']['observations'],[])
            else:self.assertTrue(data['local_feedback']['event_persisted'])
        first=json.loads((prefix/'go-hook-2026-10-06-first.json').read_text())
        repeat=json.loads((prefix/'go-hook-2026-10-06-repeat.json').read_text())
        self.assertEqual(first['local_feedback']['go_syntax']['files'][0]['task_id'],repeat['local_feedback']['go_syntax']['files'][0]['task_id'])
        self.assertEqual(first['local_feedback']['go_syntax']['files'][0]['native']['version'],'go1.23.4')
        host=json.loads((prefix/'go-hook-2026-10-06-claude.json').read_text())
        context=host['hookSpecificOutput']['additionalContext']
        self.assertIn('Go 第 2 行',context);self.assertIn('UTF-8字节',context);self.assertIn('--go-tool',context)
        self.assertNotIn('HOST_SECRET',context);self.assertLessEqual(len(context),1200)

    def test_native_first_confirmation_protocol(self):
        report=json.loads((ROOT/'tests/acceptance/evidence/go-hook-2026-10-06-native-confirmation.json').read_text())
        v=validator('syntax-confirmation-observation-v0.10.schema.json');v.validate(report)
        for key,value in [('language','ruby'),('schema_version','0.9.0'),('coverage_proven',True)]:
            bad=copy.deepcopy(report);bad[key]=value;self.assertFalse(v.is_valid(bad))
        for key,value in [('rule_id','go.vet'),('column_unit','character')]:
            bad=copy.deepcopy(report);bad['native_evidence']['native']['diagnostics'][0][key]=value;self.assertFalse(v.is_valid(bad))
        self.assertFalse(validator('syntax-confirmation-observation-v0.9.schema.json').is_valid(report))

    def test_offline_npm_installed_workflow_reports(self):
        data=json.loads((ROOT/'tests/acceptance/evidence/go-npm-repair-2026-10-06.json').read_text())
        schemas={'initialized':'init-plan.schema.json','first':'hook-execution-feedback-v0.22.schema.json','repeat':'hook-execution-feedback-v0.22.schema.json','recurrence':'hook-execution-feedback-v0.22.schema.json','next':'repair-brief-preview-v0.18.schema.json','present':'hook-execution-feedback.schema.json','repaired':'hook-execution-feedback.schema.json','failed':'hook-execution-feedback.schema.json'}
        for name,schema in schemas.items():validator(schema).validate(data[name])
        id=data['first']['local_feedback']['go_syntax']['files'][0]['task_id']
        self.assertEqual(data['next']['repair_brief']['task_id'],id)
        self.assertIn('--go-tool',data['next']['repair_brief']['recheck_argv'])
        self.assertEqual(data['recurrence']['local_feedback']['go_syntax']['files'][0]['task_id'],id)
        self.assertEqual(data['repaired']['local_feedback']['observation'],'candidate_absent_unverified_policy')
        self.assertEqual(data['failed']['reason'],'write_failed')
        self.assertNotIn('PRIVATE_MESSAGE',data['conversation']['hookSpecificOutput']['additionalContext'])

if __name__=='__main__':unittest.main()
