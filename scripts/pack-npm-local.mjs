#!/usr/bin/env node

// Build a local, single-platform npm package from an already-built Rust binary.
// This script never downloads a binary or runs project checks.
import { execFileSync, spawnSync } from 'node:child_process';
import { copyFileSync, chmodSync, existsSync, mkdirSync, mkdtempSync, readFileSync, renameSync, rmSync, writeFileSync } from 'node:fs';
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

const publishable = process.argv[2] === '--public';
if (publishable && host !== 'darwin-arm64') {
  throw new Error(`The first @partme.ai/codeguard release is certified only for darwin-arm64, not ${host}`);
}
const binaryArgument = publishable ? process.argv[3] : process.argv[2];
if (process.argv.length > (publishable ? 4 : 3)) {
  throw new Error('Usage: pack-npm-local.mjs [--public] [binary-path]');
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
    files: publishable ? ['codeguard.cjs', `native/${nativeName}`, 'README.md', 'LICENSE', 'NOTICE'] : ['codeguard.cjs', `native/${nativeName}`],
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
  const finalName = `codeguard-${publishable ? 'public-' : ''}${identity.cli_version}-${host}.tgz`;
  const finalPath = path.join(outDir, finalName);
  if (existsSync(finalPath)) rmSync(finalPath);
  renameSync(path.join(outDir, filename), finalPath);
  process.stdout.write(`${finalPath}\n`);
} finally {
  rmSync(stage, { recursive: true, force: true });
}
