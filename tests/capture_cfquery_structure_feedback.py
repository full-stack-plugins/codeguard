"""开发期真实命令捕获，使用已构建的 WASM CLI；不参与产品运行时。"""
import json,subprocess,tempfile,sys,os,hashlib
from pathlib import Path
repo=Path(__file__).resolve().parents[1];sys.path.insert(0,str(repo/'tests'))
from zig_aggregate_feedback_schema import validator
binary=repo/'target/debug/codeguard';records={}
with tempfile.TemporaryDirectory(prefix='cg-cfquery-evidence-') as d:
 root=Path(d).resolve()
 def run(args,data=None,code=3):
  o=subprocess.run([str(binary),*args],input=None if data is None else json.dumps(data),capture_output=True,text=True,env=dict(os.environ,PATH='/no/tools'),timeout=45)
  assert o.returncode==code,(o.returncode,o.stderr,o.stdout)
  return json.loads(o.stdout)
 for label,sql in [('accepted','SELECT FROM users'),('distinct','SELECT DISTINCT FROM users')]:
  path=root/(label+'.sql');path.write_text(sql)
  r=run(['grammar','probe','cfquery',str(path),'--format=json'])
  validator('grammar-probe-v0.4.schema.json' if label=='distinct' else 'grammar-probe-v0.1.schema.json').validate(r)
  records[label]=r
 run(['init',str(root),'--apply','--format=json'])
 source='<!--- 注释 --->\n<cfquery name="q">\nSELECT DISTINCT FROM users\n</cfquery>\n'
 (root/'query.cfm').write_text(source)
 event={'schema_version':'1.0.0','report_type':'hook_trigger_request','input':{'event':'file_changed','changed_paths':['query.cfm'],'task_id':None,'write_outcome':'confirmed','host_claims_blocking':False}}
 args=['hook','execute',str(root),'--timeout','30s','--format=json']
 for label in ['first','repeat']:
  r=run(args,event);validator('hook-execution-feedback-v0.23.schema.json').validate(r);records[label]=r
 r=run(['check','all',str(root),'--format=json']);validator('check-feedback-v0.53.schema.json').validate(r);records['check']=r
 for path in (root/'.codeguard/reports').glob('syntax-confirm-*.json'):
  r=json.loads(path.read_text())
  if r['language']=='cfquery':validator('syntax-confirmation-observation-v0.11.schema.json').validate(r);records['confirmation']=r
 records['next']=run(['next',str(root),'--format=json'],code=0)
 host={'hook_event_name':'PostToolUse','cwd':str(root),'tool_name':'Edit','tool_input':{'file_path':str(root/'query.cfm'),'new_string':'HOST_SECRET'},'tool_response':{'success':True}}
 records['conversation']=run(['hook','claude','post-tool-use',str(root),'--timeout','30s','--format=json'],host,0)
 context=records['conversation']['hookSpecificOutput']['additionalContext']
 assert 'codeguard.cfquery.distinct_projection' in context and 'codeguard.python.required_suite' not in context,context
 assert 'HOST_SECRET' not in context and len(context)<=1200,context
(repo/'tests/acceptance/evidence/cfquery-structure-2026-10-06.json').write_text(json.dumps({'scope':'local_static_structure_candidate_only','binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'records':records},ensure_ascii=False,indent=2)+'\n')
print('CFQuery probe, hook, project, task and conversation capture/schema PASS')
