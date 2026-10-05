"""开发期聚合结构观察的实际协议与任务回归。"""
import copy
import json
import os
import pathlib
import subprocess
import tempfile
import unittest
from zig_aggregate_feedback_schema import ROOT, validator

class CheckPythonStructure(unittest.TestCase):
    def command(self,*args,request=None):
        out=subprocess.run([str(ROOT/'target/debug/codeguard'),*args],env={**os.environ,'PATH':''},input=None if request is None else json.dumps(request).encode(),capture_output=True,timeout=130,check=False)
        self.assertEqual(out.returncode,3,out.stderr.decode())
        return json.loads(out.stdout)
    def test_actual_aggregate_protocol_and_forged_structure_import(self):
        with tempfile.TemporaryDirectory(prefix='cg-check-structure-schema-') as directory:
            root=pathlib.Path(directory).resolve()
            (root/'app.py').write_text('def run():\n',encoding='utf-8')
            self.command('init',str(root),'--apply','--format=json')
            report=self.command('check','python',str(root),'--format=json')
            validator('check-feedback-v0.48.schema.json').validate(report)
            self.assertFalse(validator('check-feedback-v0.47.schema.json').is_valid(report))
            task=report['syntax_tasks']['tasks'][0]['task_id']
            hook=self.command('hook','execute',str(root),'--timeout=30s','--format=json',request={'schema_version':'1.0.0','report_type':'hook_trigger_request','input':{'event':'file_changed','changed_paths':['app.py'],'task_id':None,'write_outcome':'confirmed','host_claims_blocking':False}})
            errors=list(validator('hook-execution-feedback-v0.17.schema.json').iter_errors(hook))
            def leaves(error):
                if error.context:
                    return [leaf for child in error.context for leaf in leaves(child)]
                return [(list(error.absolute_schema_path),error.message)]
            self.assertEqual(len(errors),0,[leaf for error in errors for leaf in leaves(error)])
            self.assertFalse(validator('hook-execution-feedback-v0.16.schema.json').is_valid(hook))
            self.assertEqual(hook['local_feedback']['next_action'],'require_native_lint_confirmation')
            self.assertEqual(hook['local_feedback']['candidate_recovery_count'],0)
            self.assertEqual(hook['local_feedback']['candidate_structure_count'],1)
            self.assertEqual(hook['local_feedback']['syntax_tasks']['tasks'][0]['task_id'],task)

            confirmation=next(r for r in (json.loads(p.read_text()) for p in (root/'.codeguard/reports').glob('*.json')) if r['report_type']=='syntax_confirmation_observation')
            schema=validator('syntax-confirmation-observation-v0.7.schema.json')
            schema.validate(confirmation)
            self.assertFalse(validator('syntax-confirmation-observation-v0.3.schema.json').is_valid(confirmation))
            self.assertEqual(confirmation['blocker_id'],task)
            self.assertEqual(confirmation['observations'][0]['recoveries'],[])
            for field,value in [('rule_sha256','0'*64),('start_byte',999)]:
                changed=copy.deepcopy(confirmation)
                changed['run_id']='syntax-confirm-999-'+('1' if field=='rule_sha256' else '2')
                changed['observations'][0]['structural_observations'][0][field]=value
                (root/'.codeguard/reports'/f"{changed['run_id']}.json").write_text(json.dumps(changed))
            sync=self.command('work','sync',str(root),'--format=json')
            self.assertEqual(sync['failed_reports'],2)
            self.assertEqual(sync['new_blockers'],0)
            for field,value in [('delivery_decision','allow'),('language','rust')]:
                changed=copy.deepcopy(confirmation);changed[field]=value
                self.assertFalse(schema.is_valid(changed))
            changed=copy.deepcopy(report)
            changed['syntax_candidates']['observations'][0]['structural_observations'][0]['kind']='ERROR'
            self.assertFalse(validator('check-feedback-v0.48.schema.json').is_valid(changed))
            human=subprocess.run([str(ROOT/'target/debug/codeguard'),'check','python',str(root)],env={**os.environ,'PATH':''},capture_output=True,timeout=130,check=False)
            self.assertEqual(human.returncode,3)
            self.assertIn('codeguard.python.required_suite',human.stdout.decode())

if __name__=='__main__':unittest.main()
