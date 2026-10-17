"""核验受控Claude协议的文档摘要；不授予已安装宿主或生产资格。"""
import hashlib
import json
from pathlib import Path
root=Path(__file__).resolve().parents[1]
source=hashlib.sha256((root/'crates/codeguard-cli/tests/claude_hook_cli.rs').read_bytes()).hexdigest()
count=0
for mode in ['default','wasm']:
    for language in ['c','cpp']:
        evidence=json.loads((root/f'tests/acceptance/evidence/c-family-host-{mode}-{language}.json').read_text())
        assert evidence['qualification']=='not_granted'
        assert evidence['test_source_sha256']==source
        output=evidence['output'];context=output['hookSpecificOutput']['additionalContext']
        assert output['hookSpecificOutput']['hookEventName']=='PostToolUse'
        assert len(context)<=1200 and 'SECRET' not in context
        assert '原生文档规则 clang.' in context
        assert 'codeguard.documentation.function_structure_required' in context
        assert '文档任务 CG-' in context and '交付未评估' in context
        count+=1
result={'evidence_kind':'development_c_family_documentation_host_context_validation','qualification':'not_granted','contexts_valid':count,'installed_host_acceptance':False}
(root/'tests/acceptance/evidence/c-family-host-validation.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result))
