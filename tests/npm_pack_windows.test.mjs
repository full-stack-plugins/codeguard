import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { npmCommand, packageTarget } from '../scripts/pack-npm-local.mjs';

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
    assert.deepEqual(npmCommand({platform:'win32', nodeExecutable:node, searchPath:''}), {command:node,args:[cli]});
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
