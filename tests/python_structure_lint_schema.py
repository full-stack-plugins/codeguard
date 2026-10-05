"""开发期Python结构候选与工作台协议实测；不作为产品运行时。"""
import copy
import json
import os
import pathlib
import subprocess
import tempfile
import unittest
from zig_aggregate_feedback_schema import ROOT, validator

class PythonStructureLint(unittest.TestCase):
    def command(self, *args, empty_path=False):
        env=os.environ.copy()
        if empty_path: env['PATH']=''
        out=subprocess.run([str(ROOT/'target/debug/codeguard'), *args], env=env, capture_output=True, timeout=130, check=False)
        self.assertEqual(out.returncode,3,out.stderr.decode())
        self.assertEqual(out.stderr,b'')
        return json.loads(out.stdout)
    def test_actual_unbound_bound_and_confirmation_versions(self):
        with tempfile.TemporaryDirectory(prefix='cg-python-structure-lint-') as directory:
            root=pathlib.Path(directory).resolve()
            (root/'broken.py').write_text('def run():\n',encoding='utf-8')
            unbound=self.command('lint','python',str(root),'--file','broken.py','--format=json',empty_path=True)
            validator('python-lint-feedback-v0.16.schema.json').validate(unbound)
            self.assertFalse(validator('python-lint-feedback-v0.14.schema.json').is_valid(unbound))
            self.command('init',str(root),'--apply','--format=json')
            bound=self.command('lint','python',str(root),'--file','broken.py','--format=json',empty_path=True)
            schema=validator('python-lint-feedback-v0.17.schema.json')
            schema.validate(bound)
            self.assertFalse(validator('python-lint-feedback-v0.15.schema.json').is_valid(bound))
            report=next(r for r in (json.loads(p.read_text()) for p in (root/'.codeguard/reports').glob('*.json')) if r['report_type']=='python_syntax_confirmation_observation')
            confirmation=validator('python-syntax-confirmation-observation-v0.2.schema.json')
            confirmation.validate(report)
            self.assertFalse(validator('python-syntax-confirmation-observation-v0.1.schema.json').is_valid(report))
            self.assertEqual(bound['setup']['task_id'],report['blocker_id'])
            self.assertEqual(report['observations'],[])
            self.assertEqual(report['structural_observations'],bound['syntax_precheck']['structural_observations'])
            for field,value in [('delivery_decision','allow'),('schema_version','0.1.0'),('structural_observations',[])]:
                forged=copy.deepcopy(report);forged[field]=value
                self.assertFalse(confirmation.is_valid(forged),field)
            forged=copy.deepcopy(report);forged['structural_observations'][0]['kind']='ERROR'
            self.assertFalse(confirmation.is_valid(forged))
            human=subprocess.run([str(ROOT/'target/debug/codeguard'),'lint','python',str(root),'--file','broken.py'],env={**os.environ,'PATH':''},capture_output=True,timeout=130,check=False)
            self.assertEqual(human.returncode,3)
            self.assertIn('codeguard.python.required_suite',human.stdout.decode())

if __name__=='__main__':unittest.main()
