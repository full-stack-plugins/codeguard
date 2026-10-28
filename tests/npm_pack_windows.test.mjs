import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync, rmSync, realpathSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { npmCommand, packageTarget, inventoryFailureSummary } from '../scripts/pack-npm-local.mjs';

test('private Windows x64 is explicit; unknown targets remain rejected', () => {
  assert.equal(packageTarget('win32-x64'), 'windows_x86_64');
  assert.equal(packageTarget('win32-arm64'), undefined);
});
test('Windows npm invocation is node plus installed npm-cli.js, never a shell', () => {
  const root = mkdtempSync(path.join(os.tmpdir(), 'cg-npm launcher & spaces-'));
  try {
    const cli = path.join(root, 'node_modules', 'npm', 'bin', 'npm-cli.js');
    mkdirSync(path.dirname(cli), { recursive: true });
    writeFileSync(cli, '// fixture');
    writeFileSync(path.join(root, 'node_modules', 'npm', 'package.json'), '{"name":"npm"}');
    const node = path.join(root, 'node.exe');
    assert.deepEqual(npmCommand({platform:'win32', nodeExecutable:node, searchPath:''}), {command:node,args:[realpathSync(cli)]});
    rmSync(cli);
    assert.throws(() => npmCommand({platform:'win32', nodeExecutable:node, searchPath:''}), /npm-cli.js/);
  } finally { rmSync(root,{recursive:true,force:true}); }
});

if (`${process.platform}-${process.arch}` !== 'darwin-arm64') {
  test('public packaging remains rejected before inspecting any binary', () => {
    const script = fileURLToPath(new URL('../scripts/pack-npm-local.mjs', import.meta.url));
    const result = spawnSync(process.execPath,[script,'--public','/nonexistent/codeguard'],{encoding:'utf8',shell:false});
    assert.notEqual(result.status,0);
    assert.match(result.stderr,/certified only for darwin-arm64/);
  });
}

test('failed WASM inventory diagnostics are bounded and preserve the actual reason', () => {
  const expected = new Map([['zig','a'.repeat(64)]]);
  const report = {report_type:'grammar_status',candidate_count:32,released_count:0,
    assets:Array.from({length:32},()=>({language:'zig',candidate_wasm_sha256:'b'.repeat(64)}))};
  const summary = inventoryFailureSummary({status:4,signal:null,stderr:'来源补丁字节不符'.repeat(500),stdout:'x'.repeat(20000)},report,expected);
  assert.equal(summary.exit_status,4);
  assert.equal(summary.asset_count,32);
  assert.equal(summary.mismatches.length,4);
  assert.equal(summary.mismatches[0].actual,'b'.repeat(64));
  assert.match(summary.stderr,/来源补丁字节不符/);
  assert(Buffer.byteLength(JSON.stringify(summary),'utf8') < 2048);
  assert.equal(inventoryFailureSummary({status:null,error:{code:'ENOENT'}},null,expected).spawn_error,'ENOENT');
});
