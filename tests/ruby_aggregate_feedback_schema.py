"""Ruby项目检查的真实输出与封闭报告协议验收。"""
import copy
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from zig_aggregate_feedback_schema import ROOT, validator

class RubyAggregateFeedback(unittest.TestCase):
    def test_archived_actual_reports(self):
        paths=sorted((ROOT/'tests/acceptance/evidence').glob('ruby-aggregate-2026-10-05-*.json'))
        self.assertGreaterEqual(len(paths),4)
        for p in paths:
            data=json.loads(p.read_text());v=validator('check-feedback-v0.51.schema.json');v.validate(data)
            for key,value in [('schema_version','2.0.0'),('delivery_decision','allow'),('authority','trusted'),('approved',True)]:
                bad=copy.deepcopy(data);bad[key]=value;self.assertFalse(v.is_valid(bad))
            if data['native_results']['ruby_lint']['files'][0]['native']['diagnostics']:
                bad=copy.deepcopy(data);bad['native_results']['ruby_lint']['files'][0]['native']['diagnostics'][0]['column']=1
                self.assertFalse(v.is_valid(bad))
    def test_actual_missing_and_invalid_selected_ruby(self):
        binary=os.environ.get('CODEGUARD_RUBY_BIN',str(ROOT/'target/debug/codeguard'))
        with tempfile.TemporaryDirectory(prefix='cg-ruby-project-schema-') as directory:
            root=Path(directory);(root/'app.rb').write_text('def f(\n')
            for extra in [[],['--ruby-tool','/missing/ruby']]:
                out=subprocess.run([binary,'check','ruby',str(root),'--format=json',*extra],env=dict(os.environ,PATH=''),capture_output=True,timeout=35)
                self.assertEqual(out.returncode,3,out.stderr.decode());data=json.loads(out.stdout);validator('check-feedback-v0.51.schema.json').validate(data)
                self.assertEqual(data['native_results']['ruby_lint']['files'][0]['native']['status'],'incomplete')
    def test_empty_native_preferred_candidates_are_not_structural_claims(self):
        paths=list((ROOT/'tests/acceptance/evidence').glob('ruby-aggregate-2026-10-05-*-native.json'))
        self.assertTrue(paths)
        for path in paths:
            data=json.loads(path.read_text());self.assertEqual(data['syntax_candidates']['observations'],[])
            self.assertEqual(data['native_results']['ruby_lint']['files'][0]['native']['status'],'diagnostics_observed')
            validator('check-feedback-v0.51.schema.json').validate(data)

if __name__=='__main__':unittest.main()
