#!/usr/bin/env node
'use strict';
const { spawnSync } = require('node:child_process');
let platform = `${process.platform}-${process.arch}`;
if (process.platform === 'linux') {
  platform += process.report.getReport().header.glibcVersionRuntime ? '-gnu' : '-musl';
}
try {
  const binary = require.resolve(`canisend-cli-${platform}/canisend${process.platform === 'win32' ? '.exe' : ''}`);
  const result = spawnSync(binary, process.argv.slice(2), { stdio: 'inherit' });
  if (result.error) throw result.error;
  if (result.signal) process.kill(process.pid, result.signal);
  else process.exitCode = result.status;
} catch (error) {
  console.error(`CanISend could not start for ${platform}. Install with optional dependencies enabled.\n${error.message}`);
  process.exitCode = 1;
}
