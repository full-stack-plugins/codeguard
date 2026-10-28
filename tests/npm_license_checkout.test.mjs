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
