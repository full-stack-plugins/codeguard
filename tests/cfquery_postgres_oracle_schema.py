"""有明确数据库方言的原生语法对照，不授予通用SQL或grammar资格。"""
import copy
import hashlib
import json
import unittest
from zig_aggregate_feedback_schema import ROOT, validator

class CfqueryPostgresOracle(unittest.TestCase):
    def test_native_context_and_same_source_worker_evidence(self):
        prefix=ROOT/'tests/acceptance/evidence'
        data=json.loads((prefix/'cfquery-postgres-oracle-2026-10-06.json').read_text())
        v=validator('cfquery-postgres-development-oracle-v0.1.schema.json');v.validate(data)
        for key,value in [('delivery_decision','allow'),('grammar_qualified',True),('independent_holdout',True),('network','project_database'),('unknown',True)]:
            bad=copy.deepcopy(data);bad[key]=value;self.assertFalse(v.is_valid(bad))
        bad=copy.deepcopy(data);bad['cases'][0]['native_valid']=False;self.assertFalse(v.is_valid(bad))
        bad=copy.deepcopy(data);bad['cases'][1]=bad['cases'][0];self.assertFalse(v.is_valid(bad))
        for case in data['cases']:
            self.assertEqual(hashlib.sha256(case['source'].encode()).hexdigest(),case['source_sha256'])
            report=json.loads((prefix/(case['id']+'-grammar-2026-10-06.json')).read_text())
            validator('grammar-probe-v0.1.schema.json').validate(report)
            self.assertEqual(report['source_sha256'],case['source_sha256'])
            self.assertEqual(report['recoveries'],[])
            self.assertFalse(report['grammar_qualified'])
        corpus=json.loads((ROOT/'tests/fixtures/grammar_regression_v0_2.json').read_text())
        pending=next(c for c in corpus['cases'] if c['id']=='cfquery-missing_select_list')
        self.assertEqual(pending['label'],'pending');self.assertEqual(pending['cohort'],'provisional_syntax')

if __name__=='__main__':unittest.main()
