"""Ruby六类别候选研究档案的封闭schema验收；不运行候选命令。"""
import copy
import json
import unittest
from zig_aggregate_feedback_schema import ROOT, validator

class RubyCandidateSchema(unittest.TestCase):
    def test_actual_bundled_profile_and_false_capability_mutations(self):
        data=json.loads((ROOT/'rulepacks/ruby_static_candidate_v1.json').read_text())
        v=validator('ruby-static-candidate.schema.json');v.validate(data)
        mutations=[(['language'],'go'),(['runtime_validation'],'verified'),(['platform_validation'],'verified'),(['categories',0,'status'],'implemented'),(['categories',0,'applicability'],'not_applicable'),(['categories',3,'requires_database_freshness'],False),(['categories',4,'tools',1,'scope'],'all_ruby_projects'),(['categories',5,'tools',0,'scope'],'all_ruby_projects'),(['categories',1,'tools',0,'candidate_argv'],['ruby','-c','app.rb']),(['categories',1,'tools',1,'tool_id'],'rubocop_documentation'),(['categories',3,'tools',0,'candidate_argv'],['bundle-audit','check','--update']),(['categories',0,'tools',0,'candidate_version'],'latest')]
        for path,value in mutations:
            bad=copy.deepcopy(data);node=bad
            for key in path[:-1]:node=node[key]
            node[path[-1]]=value
            self.assertFalse(v.is_valid(bad),path)
        for path in [('categories',),('runtime_dialects',),('platforms',),('target_version_inputs',)]:
            bad=copy.deepcopy(data);bad[path[0]].pop();self.assertFalse(v.is_valid(bad),path)
        bad=copy.deepcopy(data);bad['approved']=True;self.assertFalse(v.is_valid(bad))
        bad=copy.deepcopy(data);bad['categories'][1]['category']='lint';self.assertFalse(v.is_valid(bad))

if __name__=='__main__':unittest.main()
