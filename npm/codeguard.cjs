#!/usr/bin/env node
'use strict';

// npm only supplies the command entry point. All checks run in the Rust binary.
const { spawnSync } = require('node:child_process');
const path = require('node:path');

const executable = path.join(__dirname, 'native', process.platform === 'win32' ? 'codeguard.exe' : 'codeguard');
const result = spawnSync(executable, process.argv.slice(2), {
  cwd: process.cwd(),
  env: process.env,
  stdio: 'inherit',
  shell: false,
  windowsHide: true,
});

if (result.error) {
  process.stderr.write(`codeguard: bundled Rust executable could not start: ${result.error.message}\n`);
  process.exitCode = 4;
} else if (result.signal) {
  const signalNumber = require('node:os').constants.signals[result.signal];
  process.exitCode = signalNumber ? 128 + signalNumber : 4;
} else {
  process.exitCode = result.status ?? 4;
}
