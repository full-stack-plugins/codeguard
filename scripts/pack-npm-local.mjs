#!/usr/bin/env node

// Build a local, single-platform npm package from an already-built Rust binary.
// This script never downloads a binary or runs project checks.
import { execFileSync, spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { copyFileSync, chmodSync, existsSync, mkdirSync, mkdtempSync, readFileSync, realpathSync, renameSync, rmSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const targetByHost = new Map([
  ['darwin-arm64', 'macos_arm64'],
  ['darwin-x64', 'macos_x86_64'],
  ['linux-x64', 'linux_x86_64'],
  ['linux-arm64', 'linux_aarch64'],
]);
const host = `${process.platform}-${process.arch}`;
const expectedTarget = targetByHost.get(host);
if (!expectedTarget) {
  throw new Error(`No verified Codeguard package target for ${host}`);
}

const options = new Set();
let binaryArgument;
for (const argument of process.argv.slice(2)) {
  if (argument === '--public' || argument === '--require-wasm') {
    if (options.has(argument)) throw new Error(`Duplicate option: ${argument}`);
    options.add(argument);
  } else if (argument.startsWith('--') || binaryArgument) {
    throw new Error('Usage: pack-npm-local.mjs [--public] [--require-wasm] [binary-path]');
  } else {
    binaryArgument = argument;
  }
}
const publishable = options.has('--public');
const requireWasm = publishable || options.has('--require-wasm');
const grammarLicenses = new Map();
if (publishable && host !== 'darwin-arm64') {
  throw new Error(`The first @partme.ai/codeguard release is certified only for darwin-arm64, not ${host}`);
}
const binary = binaryArgument
  ? path.resolve(binaryArgument)
  : path.join(root, 'target', 'release', process.platform === 'win32' ? 'codeguard.exe' : 'codeguard');
if (!existsSync(binary)) {
  throw new Error(`Rust binary missing: ${binary}. Run cargo build --release --locked -p codeguard-cli first.`);
}

const versionResult = execFileSync(binary, ['--version', '--format', 'json'], {
  cwd: root,
  encoding: 'utf8',
  timeout: 10_000,
  maxBuffer: 128 * 1024,
});
const identity = JSON.parse(versionResult);
const workspaceVersion = readFileSync(path.join(root, 'Cargo.toml'), 'utf8')
  .match(/^version\s*=\s*"([^"]+)"/m)?.[1];
if (identity.report_type !== 'version' || identity.target !== expectedTarget || identity.cli_version !== workspaceVersion) {
  throw new Error(`Binary identity must match host ${expectedTarget} and workspace version ${workspaceVersion ?? '(missing)'}`);
}
if (publishable) {
  const sourceSha = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim();
  const sourceChanges = execFileSync('git', ['status', '--porcelain=v1', '--untracked-files=normal'], {
    cwd: root, encoding: 'utf8', maxBuffer: 128 * 1024,
  });
  if (sourceChanges.trim()) {
    throw new Error('Public package requires a clean source checkout');
  }
  if (identity.build_identity !== sourceSha) {
    throw new Error('Public binary build identity must equal the checked-out source commit');
  }
}

if (requireWasm) {
  const manifest = JSON.parse(readFileSync(path.join(root, 'grammars', 'manifest.json'), 'utf8'));
  const expected = new Map(manifest.assets.map(asset => [asset.language, asset.sha256]));
  if (expected.size !== 32 || manifest.assets.length !== 32) {
    throw new Error('WASM capability probe failed: local manifest must contain 32 distinct assets');
  }
  const licenseEntries = [
    [manifest.codegraph_license, manifest.codegraph_license_sha256],
    ...manifest.assets.map(asset => [asset.license, asset.license_sha256]),
  ];
  for (const [relativePath, expectedSha256] of licenseEntries) {
    if (typeof relativePath !== 'string' ||
        !/^(?:LICENSE\.codegraph|[a-z0-9_-]+\/LICENSE)$/.test(relativePath) ||
        typeof expectedSha256 !== 'string' || !/^[a-f0-9]{64}$/.test(expectedSha256)) {
      throw new Error('WASM license gate failed: invalid manifest license identity');
    }
    if (grammarLicenses.has(relativePath) && grammarLicenses.get(relativePath) !== expectedSha256) {
      throw new Error(`WASM license gate failed: conflicting hash for ${relativePath}`);
    }
    const source = path.join(root, 'grammars', relativePath);
    if (!existsSync(source) || createHash('sha256').update(readFileSync(source)).digest('hex') !== expectedSha256) {
      throw new Error(`WASM license gate failed: missing or changed ${relativePath}`);
    }
    grammarLicenses.set(relativePath, expectedSha256);
  }
  const inventory = spawnSync(binary, ['grammar', 'status', '--format=json'], {
    cwd: root, encoding: 'utf8', timeout: 30_000, maxBuffer: 1024 * 1024, shell: false,
  });
  let report;
  try { report = JSON.parse(inventory.stdout); } catch { report = null; }
  if (inventory.error || inventory.status !== 3 || report?.candidate_count !== 32 ||
      !Array.isArray(report.assets) || report.assets.length !== 32 ||
      new Set(report.assets.map(asset => asset.language)).size !== 32 ||
      report.released_count !== 0 || report.delivery_decision !== 'not_evaluated' ||
      report.assets.some(asset => !expected.has(asset.language) || expected.get(asset.language) !== asset.candidate_wasm_sha256)) {
    throw new Error('WASM capability probe failed: 32 pinned asset identities are not present');
  }
  const probeDir = realpathSync(mkdtempSync(path.join(os.tmpdir(), 'codeguard-wasm-pack-probe-')));
  try {
    const source = path.join(probeDir, 'sample.zig');
    const sourceText = 'const Empty = struct {};\n';
    writeFileSync(source, sourceText);
    const observed = spawnSync(binary, ['grammar', 'probe', 'zig', source, '--format=json'], {
      cwd: root, encoding: 'utf8', timeout: 120_000, maxBuffer: 1024 * 1024, shell: false,
    });
    let candidate;
    try { candidate = JSON.parse(observed.stdout); } catch { candidate = null; }
    if (observed.error || observed.status !== 3 || candidate?.report_type !== 'grammar_candidate_probe' ||
        candidate.language !== 'zig' || candidate.grammar_sha256 !== expected.get('zig') ||
        candidate.source_sha256 !== createHash('sha256').update(sourceText).digest('hex') ||
        candidate.grammar_qualified !== false || candidate.precheck?.status !== 'incomplete' ||
        candidate.precheck?.checked_files !== 1 || !Array.isArray(candidate.recoveries) ||
        candidate.native?.status !== 'not_run' || candidate.delivery_decision !== 'not_evaluated') {
      throw new Error('WASM capability probe failed: pinned Zig worker did not produce an incomplete candidate');
    }
  } finally {
    rmSync(probeDir, { recursive: true, force: true });
  }
}

const stage = mkdtempSync(path.join(os.tmpdir(), 'codeguard-npm-'));
const outDir = path.join(root, 'release', 'npm');
try {
  mkdirSync(path.join(stage, 'native'));
  copyFileSync(path.join(root, 'npm', 'codeguard.cjs'), path.join(stage, 'codeguard.cjs'));
  if (publishable) {
    copyFileSync(path.join(root, 'npm', 'README.md'), path.join(stage, 'README.md'));
    copyFileSync(path.join(root, 'LICENSE'), path.join(stage, 'LICENSE'));
    copyFileSync(path.join(root, 'NOTICE'), path.join(stage, 'NOTICE'));
  }
  if (requireWasm) {
    for (const relativePath of grammarLicenses.keys()) {
      const destination = path.join(stage, 'grammar-licenses', relativePath);
      mkdirSync(path.dirname(destination), { recursive: true });
      copyFileSync(path.join(root, 'grammars', relativePath), destination);
    }
  }
  const nativeName = process.platform === 'win32' ? 'codeguard.exe' : 'codeguard';
  const packagedBinary = path.join(stage, 'native', nativeName);
  copyFileSync(binary, packagedBinary);
  if (process.platform !== 'win32') chmodSync(packagedBinary, 0o755);
  writeFileSync(path.join(stage, 'package.json'), JSON.stringify({
    name: publishable ? '@partme.ai/codeguard' : '@full-stack-plugins/codeguard',
    version: identity.cli_version,
    description: `Codeguard Rust CLI for ${host}; native static-tool orchestration`,
    ...(publishable ? { publishConfig: { access: 'public' } } : { private: true }),
    license: 'Apache-2.0',
    repository: { type: 'git', url: 'git+https://github.com/full-stack-plugins/codeguard.git' },
    os: [process.platform],
    cpu: [process.arch],
    engines: { node: '>=18' },
    bin: { codeguard: './codeguard.cjs' },
    files: [
      'codeguard.cjs', `native/${nativeName}`,
      ...(requireWasm ? ['grammar-licenses/**'] : []),
      ...(publishable ? ['README.md', 'LICENSE', 'NOTICE'] : []),
    ],
  }, null, 2) + '\n');
  chmodSync(path.join(stage, 'codeguard.cjs'), 0o755);
  mkdirSync(outDir, { recursive: true });
  const pack = spawnSync('npm', ['pack', stage, '--json', '--ignore-scripts', '--pack-destination', outDir], {
    cwd: root,
    encoding: 'utf8',
    timeout: 120_000,
    maxBuffer: 2 * 1024 * 1024,
    shell: false,
  });
  if (pack.error || pack.status !== 0) {
    throw new Error(`npm pack failed: ${pack.error?.message ?? pack.stderr.trim()}`);
  }
  const [{ filename, files }] = JSON.parse(pack.stdout);
  if (!files.some(file => file.path === `native/${nativeName}`)) {
    throw new Error('npm tarball omitted the Rust executable');
  }
  for (const relativePath of grammarLicenses.keys()) {
    if (!files.some(file => file.path === `grammar-licenses/${relativePath}`)) {
      throw new Error(`npm tarball omitted grammar license ${relativePath}`);
    }
  }
  const finalName = `codeguard-${publishable ? 'public-' : ''}${identity.cli_version}-${host}.tgz`;
  const finalPath = path.join(outDir, finalName);
  if (existsSync(finalPath)) rmSync(finalPath);
  renameSync(path.join(outDir, filename), finalPath);
  process.stdout.write(`${finalPath}\n`);
} finally {
  rmSync(stage, { recursive: true, force: true });
}
