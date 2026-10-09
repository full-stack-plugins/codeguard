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

    def test_real_workbench_captures(self):
        prefix=ROOT/'tests/acceptance/evidence'
        v=validator('shell-lint-feedback-v0.2.schema.json')
        first=None
        for name in ['first','repeat','suppressed','clean']:
            data=json.loads((prefix/f'shell-workbench-2026-10-06-{name}.json').read_text());v.validate(data)
            self.assertEqual(data['workbench']['status'],'synced_partial')
            self.assertFalse(validator('shell-lint-feedback-v0.1.schema.json').is_valid(data))
            if name=='first':first=data['workbench']['task_ids'];self.assertEqual(len(first),1)
            if name=='repeat':self.assertEqual(data['workbench']['task_ids'],first)
            bad=copy.deepcopy(data);bad['task_workflow_status']='completed';self.assertFalse(v.is_valid(bad))
            bad=copy.deepcopy(data);bad['delivery_decision']='allow';self.assertFalse(v.is_valid(bad))
        for name in ['next-suppressed','next-clean']:
            data=json.loads((prefix/f'shell-workbench-2026-10-06-{name}.json').read_text())
            validator('repair-brief-preview-v0.16.schema.json').validate(data)
            self.assertEqual(data['disposition'],'verification_required')
            self.assertEqual(data['repair_brief']['checker_id'],'shell.shellcheck')
            self.assertFalse(validator('repair-brief-preview-v0.15.schema.json').is_valid(data))
        for i in range(4):
            data=json.loads((prefix/f'shell-workbench-2026-10-06-observation-{i}.json').read_text())
            validator('shellcheck-workbench-observation-v0.1.schema.json').validate(data)
        fact=json.loads((prefix/'shell-workbench-2026-10-06-fact.json').read_text());self.assertEqual(fact['state'],'open')

    def test_real_shell_task_rechecks(self):
        prefix=ROOT/'tests/acceptance/evidence'
        v=validator('task-verification-preview-v0.24.schema.json')
        for name,outcome in [('present','still_present'),('rc-disabled','rule_coverage_requires_review'),('comment-disabled','suppression_requires_review'),('fixed','candidate_absent_unverified_policy')]:
            data=json.loads((prefix/f'shell-task-2026-10-06-{name}.json').read_text())
            v.validate(data)
            self.assertEqual(data['observation'],outcome)
            self.assertTrue(data['event_persisted'])
            for key,value in [('delivery_decision','allow'),('authority','trusted'),('observation','resolved')]:
                bad=copy.deepcopy(data);bad[key]=value;self.assertFalse(v.is_valid(bad))
            bad=copy.deepcopy(data);bad['native_scan']['unexpected']=True;self.assertFalse(v.is_valid(bad))
            bad=copy.deepcopy(data);bad['native_scan']['task_rule']=None;self.assertFalse(v.is_valid(bad))
        fact=json.loads((prefix/'shell-task-2026-10-06-fact.json').read_text())
        self.assertEqual(fact['state'],'open')

    def test_task_bound_shell_guidance_and_failed_attempts(self):
        prefix=ROOT/'tests/acceptance/evidence'
        for name in ['initial','after-0','after-1']:
            data=json.loads((prefix/f'shell-next-2026-10-06-{name}.json').read_text())
            v=validator('repair-brief-preview-v0.17.schema.json');v.validate(data)
            argv=data['repair_brief']['recheck_argv']
            self.assertEqual(argv[:4],['codeguard','task','verify',data['repair_brief']['task_id']])
            self.assertFalse(validator('repair-brief-preview-v0.16.schema.json').is_valid(data))
            bad=copy.deepcopy(data);bad['repair_brief']['recheck_argv'][1]='lint';self.assertFalse(v.is_valid(bad))
            bad=copy.deepcopy(data);bad['repair_brief']['recheck_argv'][4]='.';self.assertFalse(v.is_valid(bad))
            if name=='after-1':
                self.assertEqual(data['disposition'],'needs_decision')
                self.assertEqual(data['repair_brief']['history']['no_progress_count'],2)
        for i in range(2):
            event=json.loads((prefix/f'shell-next-2026-10-06-event-{i}.json').read_text())
            validator('task-verification-event.schema.json').validate(event)
            self.assertIsInstance(event['attempt_id'],str)
            data=json.loads((prefix/f'shell-next-2026-10-06-verify-{i}.json').read_text())
            validator('task-verification-preview-v0.24.schema.json').validate(data)
            self.assertEqual(data['native_scan']['run_id'],event['run_id'])
            self.assertEqual(data['observation'],'still_present')
        denied=json.loads((prefix/'shell-next-2026-10-06-retry-denied.json').read_text())
        validator('task-attempt-response.schema.json').validate(denied)
        self.assertEqual(denied['reason'],'no_progress_budget_exhausted')
        shown=json.loads((prefix/'shell-next-2026-10-06-show.json').read_text())
        validator('task-show-preview-v0.3.schema.json').validate(shown)
        self.assertEqual(shown['next_actions'][0],shown['task']['recheck_argv'])
        fact=json.loads((prefix/'shell-next-2026-10-06-fact.json').read_text())
        self.assertEqual(fact['state'],'open')

    def test_real_shell_project_reports(self):
        prefix=ROOT/'tests/acceptance/evidence'
        v=validator('check-feedback-v0.52.schema.json')
        for name in ['first','repeat','all','missing','unknown-dialect','unsupported']:
            data=json.loads((prefix/f'shell-project-2026-10-06-{name}.json').read_text());v.validate(data)
            scan=data['native_results']['shell_lint']
            self.assertEqual(scan['source_file_count'],2)
            self.assertFalse(scan['coverage_proven'])
            bad=copy.deepcopy(data);bad['authority']='trusted';self.assertFalse(v.is_valid(bad))
            bad=copy.deepcopy(data);bad['delivery_decision']='allow';self.assertFalse(v.is_valid(bad))
            bad=copy.deepcopy(data);bad['native_results']['shell_lint']['files'][0]['unexpected']=True;self.assertFalse(v.is_valid(bad))
            if name in ['first','repeat','all']:
                self.assertTrue(scan['local_check_complete'])
                self.assertEqual(scan['files'][0]['native']['diagnostics'][0]['rule_id'],'SC2086')
                bad=copy.deepcopy(data);bad['native_results']['shell_lint']['files'][0]['input_stable']=False;self.assertFalse(v.is_valid(bad))
            if name=='missing':self.assertEqual(scan['files'][0]['native']['reason'],'shellcheck_tool_not_found')
            if name in ['missing','unknown-dialect','unsupported']:self.assertFalse(scan['local_check_complete'])
            if name in ['unknown-dialect','unsupported']:self.assertEqual(data['next']['disposition'],'needs_decision')
        sarif=json.loads((prefix/'shell-project-2026-10-06-sarif.json').read_text())
        self.assertEqual(sarif['runs'][0]['properties']['nativeFindingCount'],2)
        self.assertFalse(sarif['runs'][0]['invocations'][0]['executionSuccessful'])
        self.assertFalse(any(r.get('locations') for r in sarif['runs'][0]['results']))

    def test_help_compatibility(self):
        data = json.loads((ROOT / 'tests/acceptance/evidence/command-help-shell-2026-10-06-default.json').read_text())
        validator('command-help-v0.4.schema.json').validate(data)
        for version in ['0.1', '0.2', '0.3']:
            self.assertFalse(validator(f'command-help-v{version}.schema.json').is_valid(data))
        self.assertIn('shell', next(c for c in data['commands'] if c['command'] == 'lint')['languages'])

if __name__ == '__main__': unittest.main()
