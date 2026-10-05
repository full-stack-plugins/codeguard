"""开发期真实探针协议回归；不作为产品运行时或语言质量认证。"""
import copy
import hashlib
import json
import pathlib
import subprocess
import tempfile
import unittest
from zig_aggregate_feedback_schema import ROOT, validator

class PythonStructureProbe(unittest.TestCase):
    def test_actual_report_and_closed_versions(self):
        with tempfile.TemporaryDirectory(prefix='cg-python-structure-schema-') as directory:
            path = pathlib.Path(directory).resolve()/'app.py'
            path.write_text('def run():\n', encoding='utf-8')
            process = subprocess.run([str(ROOT/'target/debug/codeguard'), 'grammar', 'probe', 'python', str(path), '--format=json'], capture_output=True, timeout=100, check=False)
            self.assertEqual(process.returncode, 3)
            self.assertEqual(process.stderr, b'')
            report=json.loads(process.stdout)
        schema=validator('grammar-probe-v0.2.schema.json')
        schema.validate(report)
        self.assertFalse(validator('grammar-probe-v0.1.schema.json').is_valid(report))
        self.assertEqual(report['recoveries'], [])
        self.assertEqual(report['precheck']['suspected_recoveries'], 0)
        row=report['structural_observations'][0]
        self.assertEqual(row['rule_sha256'], hashlib.sha256((ROOT/'rulepacks/python/required_suite.json').read_bytes()).hexdigest())
        for field,value in [('language','rust'),('grammar_qualified',True),('delivery_decision','allow'),('structural_observations',[])]:
            changed=copy.deepcopy(report); changed[field]=value
            self.assertFalse(schema.is_valid(changed),field)
        for field,value in [('basis','ERROR'),('rule_id','native.ruff'),('parent_syntax_kind','function_item'),('start_byte',-1)]:
            changed=copy.deepcopy(report);changed['structural_observations'][0][field]=value
            self.assertFalse(schema.is_valid(changed),field)
        changed=copy.deepcopy(report);changed['structural_observations'][0]['approved']=True
        self.assertFalse(schema.is_valid(changed))

if __name__=='__main__':unittest.main()
