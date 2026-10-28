import test from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync, rmSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
test('Windows autocrlf checkout preserves every manifest-pinned license byte', () => {
  const manifest = JSON.parse(readFileSync(path.join(root, 'grammars/manifest.json'), 'utf8'));
  assert.equal(manifest.assets.length, 32);
  const licenses = new Map([[manifest.codegraph_license, manifest.codegraph_license_sha256],
    ...manifest.assets.map(asset => [asset.license, asset.license_sha256])]);
  const scratch = mkdtempSync(path.join(os.tmpdir(), 'codeguard-license-checkout-'));
  const git = args => execFileSync('git', args, { cwd:scratch, stdio:['ignore','pipe','pipe'] });
  try {
    git(['init', '--quiet']);
    git(['config', 'core.autocrlf', 'false']);
    writeFileSync(path.join(scratch, '.gitattributes'), readFileSync(path.join(root,'.gitattributes')));
    for (const [license, sha] of licenses) {
      const relative = `grammars/${license}`;
      const blob = execFileSync('git', ['show', `HEAD:${relative}`], {cwd:root});
      assert.equal(createHash('sha256').update(blob).digest('hex'), sha, `source blob: ${license}`);
      const target = path.join(scratch,relative);
      mkdirSync(path.dirname(target), {recursive:true});
      writeFileSync(target,blob);
    }
    git(['add','--','.gitattributes','grammars']);
    for (const license of licenses.keys()) rmSync(path.join(scratch,'grammars',license));
    git(['config','core.autocrlf','true']);
    git(['checkout-index','--force','--all']);
    for (const [license, sha] of licenses) {
      const checkedOut = readFileSync(path.join(scratch,'grammars',license));
      assert.equal(createHash('sha256').update(checkedOut).digest('hex'),sha,`autocrlf checkout: ${license}`);
    }
  } finally { rmSync(scratch,{recursive:true,force:true}); }
});

test('Windows autocrlf preserves source patches checked by grammar asset verification', () => {
  const patches = new Map([
    ['vbnet/source.patch','33963ebe3ba60929fb58d90efd54f206f98a54ea13ae325d784732c828b7b93a'],
    ['cobol/source.patch','04fe11a51243ddf7d31e0b6378068c725a51463534218c43c7021761c69701e5'],
  ]);
  const scratch = mkdtempSync(path.join(os.tmpdir(), 'codeguard-patch-checkout-'));
  const git = args => execFileSync('git', args, {cwd:scratch,stdio:['ignore','pipe','pipe']});
  try {
    git(['init','--quiet']);
    git(['config','core.autocrlf','false']);
    writeFileSync(path.join(scratch,'.gitattributes'),readFileSync(path.join(root,'.gitattributes')));
    for (const [name, expected] of patches) {
      const blob = execFileSync('git',['show',`HEAD:grammars/${name}`],{cwd:root});
      assert.equal(createHash('sha256').update(blob).digest('hex'),expected);
      const target = path.join(scratch,'grammars',name);
      mkdirSync(path.dirname(target),{recursive:true});
      writeFileSync(target,blob);
    }
    git(['add','--','.gitattributes','grammars']);
    for (const name of patches.keys()) rmSync(path.join(scratch,'grammars',name));
    git(['config','core.autocrlf','true']);
    git(['checkout-index','--force','--all']);
    for (const [name,expected] of patches) {
      assert.equal(createHash('sha256').update(readFileSync(path.join(scratch,'grammars',name))).digest('hex'),expected,name);
    }
  } finally { rmSync(scratch,{recursive:true,force:true}); }
});
