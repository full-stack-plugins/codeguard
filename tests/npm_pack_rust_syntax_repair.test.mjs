import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, realpathSync, rmSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
// 夹具仅验收安装后编排；真实Rust SDK对照另有报告，不以脚本冒充语法oracle。
const rustfmtFixture = '#!/bin/sh\nif [ "$1" = --version ]; then printf "rustfmt 1.9.0-stable (fixture)\\n"; exit 0; fi\ninput=$(/bin/cat)\ncase "$input" in *"pub fn main( {"*) printf "error: expected token\\n --> <stdin>:1:1\\n" >&2; exit 1;; esac\nprintf "%s\\n" "$input"\n';
test('离线npm安装后Rust编辑、next与原SDK复检共用稳定任务', () => {
  const binary = process.env.CODEGUARD_WASM_BIN;
  assert.ok(binary, 'CODEGUARD_WASM_BIN is required');
  const packed = spawnSync(process.execPath, [path.join(root, 'scripts/pack-npm-local.mjs'), '--require-wasm', binary], { cwd: root, encoding: 'utf8', timeout: 120_000 });
  assert.equal(packed.status, 0, packed.stderr);
  const scratch = realpathSync(mkdtempSync(path.join(os.tmpdir(), 'cg-npm-rust-repair-')));
  const project = path.join(scratch, 'project');mkdirSync(project);
  try {
    const source=path.join(project,'app.rs'); const tool=path.join(scratch,'rustfmt');
    writeFileSync(source,'pub fn main( {\n');
    writeFileSync(tool,rustfmtFixture,{mode:0o700});writeFileSync(path.join(project,'Cargo.toml'),"[package]\nname='sample'\nversion='0.1.0'\nedition='2021'\n");
    function invoke(args,input,expected=3) {
      const result=spawnSync('npm',['exec','--offline','--yes','--ignore-scripts','--cache',path.join(scratch,'cache'),'--package',packed.stdout.trim(),'--','codeguard',...args,'--format=json'],{
        cwd:project,input:input===undefined?undefined:JSON.stringify(input),encoding:'utf8',timeout:120_000,maxBuffer:1024*1024,env:{...process.env,npm_config_audit:'false',npm_config_fund:'false'},
      });
      assert.equal(result.error,undefined,result.error?.message);assert.equal(result.status,expected,result.stderr);return JSON.parse(result.stdout);
    }
    const event=(kind='file_changed',id=null)=>({schema_version:'1.0.0',report_type:'hook_trigger_request',input:{event:kind,changed_paths:kind==='file_changed'?['app.rs']:[],task_id:id,write_outcome:'confirmed',host_claims_blocking:false}});
    const edit=()=>invoke(['hook','execute',project,'--rustfmt-tool',tool,'--timeout','30s'],event());
    const initialized=invoke(['init',project,'--apply']);const first=edit();
    assert.equal(first.schema_version,'0.24.0');assert.equal(first.local_feedback.rust_syntax.files[0].native.status,'diagnostics_observed');
    const id=first.local_feedback.rust_syntax.files[0].task_id;assert.match(id??'',/^CG-B-[a-f0-9]{32}$/);
    const repeat=edit();assert.equal(repeat.local_feedback.rust_syntax.files[0].task_id,id);
    const next=invoke(['next',project],undefined,0);assert.equal(next.schema_version,'0.19.0');assert.equal(next.repair_brief.task_id,id);assert.ok(next.repair_brief.recheck_argv.includes('--rustfmt-tool'));
    const conversation=invoke(['hook','claude','post-tool-use',project,'--rustfmt-tool',tool,'--timeout','30s'],{hook_event_name:'PostToolUse',cwd:project,tool_name:'Edit',tool_input:{file_path:source,new_string:'HOST_SECRET'},tool_response:{success:true}},0);
    const context=conversation.hookSpecificOutput.additionalContext;
    assert.ok(context.includes(id)&&context.includes('Rust 第 1 行')&&context.includes('--rustfmt-tool'));
    assert.ok(!context.includes('HOST_SECRET')&&!context.includes('PRIVATE_MESSAGE'));assert.ok([...context].length<=1200);
    const verify=()=>invoke(['hook','execute',project,'--rustfmt-tool',tool,'--timeout','30s'],event('repair_ready',id));
    const present=verify();assert.equal(present.local_feedback.observation,'still_blocked');assert.equal(present.local_feedback.event_persisted,true);
    writeFileSync(source,'pub fn main() {}\n');const repaired=verify();assert.equal(repaired.local_feedback.observation,'candidate_absent_unverified_policy');
    const fact=path.join(project,'.codeguard/findings',id,'finding.json');assert.equal(JSON.parse(readFileSync(fact)).state,'open');
    writeFileSync(source,'pub fn main( {\n');const recurrence=edit();assert.equal(recurrence.local_feedback.rust_syntax.files[0].task_id,id);
    const marker=path.join(scratch,'failed-write-executed');const quote="'"+marker.replaceAll("'","'\\''")+"'";
    writeFileSync(tool,`#!/bin/sh\n/usr/bin/touch ${quote}\nexit 7\n`);
    const failedEvent=event();failedEvent.input.write_outcome='failed';const failed=invoke(['hook','execute',project,'--rustfmt-tool',tool,'--timeout','30s'],failedEvent);
    assert.equal(failed.execution,'not_run');assert.equal(failed.reason,'write_failed');assert.equal(existsSync(marker),false);
    // 安装后的项目 lint 与 task-bound Clippy 复检；不在编辑事件中执行构建。
    writeFileSync(path.join(project,'Cargo.lock'),"version=4\n[[package]]\nname='sample'\nversion='0.1.0'\n");
    const cargo=path.join(scratch,'cargo');
    const warning=JSON.stringify({reason:'compiler-message',message:{level:'warning',code:{code:'clippy::needless_return'},spans:[{file_name:'app.rs',line_start:1,column_start:20,is_primary:true}]}});
    writeFileSync(cargo,`#!/bin/sh\n[ "$1" = clippy ] || exit 29\nif /usr/bin/grep -q 'return 42' app.rs; then printf '%s\\n' '${warning}'; fi\nprintf '%s\\n' '{"reason":"build-finished","success":true}'\n`,{mode:0o700});
    writeFileSync(source,'pub fn answer() -> i32 { return 42; }\n');
    const clippy=invoke(['lint','rust',project,'--cargo-tool',cargo]);
    const clippyId=clippy.next.repair_brief.task_id;
    const clippyRecheck=()=>invoke(['hook','execute',project,'--cargo-tool',cargo,'--timeout','30s'],event('repair_ready',clippyId));
    const clippyPresent=clippyRecheck();
    assert.equal(clippyPresent.schema_version,'0.26.0');
    assert.deepEqual(clippyPresent.local_feedback.native_diagnostic_positions,[{line:1,rule_id:'clippy::needless_return'}]);
    writeFileSync(source,'pub fn answer() -> i32 { 42 }\n');
    const clippyRepaired=clippyRecheck();
    assert.equal(clippyRepaired.local_feedback.observation,'candidate_absent_unverified_policy');
    assert.deepEqual(clippyRepaired.local_feedback.native_diagnostic_positions,[]);
    assert.equal(JSON.parse(readFileSync(path.join(project,'.codeguard/findings',clippyId,'finding.json'))).state,'open');
    if(process.env.CODEGUARD_NPM_RUST_ARTIFACT)writeFileSync(process.env.CODEGUARD_NPM_RUST_ARTIFACT,JSON.stringify({initialized,first,repeat,next,conversation,present,repaired,recurrence,failed,clippy,clippyPresent,clippyRepaired},null,2));
  } finally { rmSync(scratch,{recursive:true,force:true}); }
});
