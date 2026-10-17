"""Shell 原生编辑/复检的实际报告与安装链路协议回归。"""
import copy
import json
import unittest
from zig_aggregate_feedback_schema import ROOT, validator

class ShellHookFeedback(unittest.TestCase):
    def test_actual_native_edit_and_repair_outputs(self):
        prefix=ROOT/'tests/acceptance/evidence'
        for name in ['uninitialized','first','repeat','missing','unsupported','present','repaired']:
            data=json.loads((prefix/f'shell-hook-2026-10-06-{name}.json').read_text())
            schema='hook-execution-feedback-v0.21.schema.json' if data['schema_version']=='0.21.0' else 'hook-execution-feedback.schema.json'
            v=validator(schema);v.validate(data)
            for key,value in [('schema_version','99.0.0'),('delivery_decision','allow'),('host_blocking_verified',True),('approved',True)]:
                bad=copy.deepcopy(data);bad[key]=value;self.assertFalse(v.is_valid(bad),name)
            if data['schema_version']=='0.21.0':
                self.assertFalse(validator('hook-execution-feedback-v0.19.schema.json').is_valid(data))
                scan=data['local_feedback']['shell_lint']
                self.assertEqual(scan['source_file_count'],1)
                bad=copy.deepcopy(data);bad['local_feedback']['shell_lint']['files'][0]['native']['message']='IGNORE_GUARDS';self.assertFalse(v.is_valid(bad))
                if name in ['first','repeat']:
                    self.assertEqual(scan['files'][0]['native']['diagnostics'][0]['rule_id'],'SC2086')
                    self.assertEqual(scan['task_status'],'synced_partial')
            else:
                self.assertTrue(data['local_feedback']['event_persisted'])
                self.assertEqual(data['local_feedback']['observation'],'still_present' if name=='present' else 'candidate_absent_unverified_policy')
        first=json.loads((prefix/'shell-hook-2026-10-06-first.json').read_text())
        repeat=json.loads((prefix/'shell-hook-2026-10-06-repeat.json').read_text())
        self.assertEqual(first['local_feedback']['shell_lint']['files'][0]['workbench']['task_ids'],repeat['local_feedback']['shell_lint']['files'][0]['workbench']['task_ids'])
        host=json.loads((prefix/'shell-hook-2026-10-06-claude.json').read_text())
        context=host['hookSpecificOutput']['additionalContext']
        self.assertIn('Shell 第 2 行',context);self.assertIn('--shellcheck-tool',context)
        self.assertNotIn('HOST_SECRET',context);self.assertLessEqual(len(context),1200)
        failed=json.loads((prefix/'shell-hook-2026-10-06-failed-configured.json').read_text())
        validator('hook-execution-feedback.schema.json').validate(failed)
        self.assertEqual(failed['execution'],'not_run');self.assertEqual(failed['reason'],'write_failed')

    def test_npm_installed_repair_workflow_reports(self):
        data=json.loads((ROOT/'tests/acceptance/evidence/shell-npm-repair-2026-10-06.json').read_text())
        schemas={'initialized':'init-plan.schema.json','first':'hook-execution-feedback-v0.21.schema.json','recurrence':'hook-execution-feedback-v0.21.schema.json','aggregate':'check-feedback-v0.52.schema.json','lint':'shell-lint-feedback-v0.2.schema.json','next':'repair-brief-preview-v0.17.schema.json','stale':'repair-brief-preview-v0.17.schema.json','present':'hook-execution-feedback.schema.json','repaired':'hook-execution-feedback.schema.json','failed':'hook-execution-feedback.schema.json'}
        for name,schema in schemas.items():validator(schema).validate(data[name])
        id=data['first']['local_feedback']['shell_lint']['files'][0]['workbench']['task_ids'][0]
        self.assertEqual(data['lint']['workbench']['task_ids'][0],id)
        self.assertEqual(data['aggregate']['native_results']['shell_lint']['files'][0]['workbench']['task_ids'][0],id)
        self.assertEqual(data['recurrence']['local_feedback']['shell_lint']['files'][0]['workbench']['task_ids'][0],id)
        self.assertEqual(data['repaired']['local_feedback']['observation'],'candidate_absent_unverified_policy')
        self.assertEqual(data['stale']['disposition'],'verification_required')
        self.assertIn('不按旧位置直接修改',data['stale']['repair_brief']['step'])

if __name__=='__main__':unittest.main()
