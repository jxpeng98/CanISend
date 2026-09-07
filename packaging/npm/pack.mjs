// Package already staged native bundles; no downloads or publication here.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { execFileSync } from 'node:child_process';
const platforms = {
  'aarch64-apple-darwin': ['darwin', 'arm64'],
  'x86_64-apple-darwin': ['darwin', 'x64'],
  'x86_64-unknown-linux-gnu': ['linux', 'x64', 'gnu'],
  'x86_64-unknown-linux-musl': ['linux', 'x64', 'musl'],
  'x86_64-pc-windows-msvc': ['win32', 'x64'],
};
const [destination, ...bundles] = process.argv.slice(2);
if (!destination || !bundles.length) throw Error('usage: node pack.mjs NEW_OUTPUT STAGED_BUNDLE...');
const output = path.resolve(destination);
const seen = new Set();
const inputs = bundles.map(bundle => {
  const identity = JSON.parse(fs.readFileSync(path.join(bundle, 'RELEASE.json')));
  const target = fs.readFileSync(path.join(bundle, 'TARGET'), 'utf8').trim();
  if (!identity.ok || identity.data.product !== 'canisend' || identity.data.target !== target || !platforms[target]) throw Error('invalid bundle identity');
  if (seen.has(target)) throw Error(`duplicate target: ${target}`);
  seen.add(target);
  return { bundle, target, ...identity.data };
});
const version = inputs[0].version;
if (!/^\d+\.\d+\.\d+(?:-[a-zA-Z0-9.-]+)?$/.test(version) || inputs.some(x => x.version !== version || x.git_revision !== inputs[0].git_revision)) throw Error('bundle versions or revisions differ');
function platformMetadata(target) {
  const [os, cpu, libc] = platforms[target];
  return { os: [os], cpu: [cpu], ...(libc ? { libc: [libc === 'gnu' ? 'glibc' : libc] } : {}) };
}
fs.mkdirSync(output); // Refuse overwrite.
const directory = path.join(output, 'canisend');
fs.mkdirSync(directory);
function copyFile(source, destination) {
  if (!fs.lstatSync(source).isFile()) throw Error(`not a regular file: ${source}`);
  fs.copyFileSync(source, destination);
}
for (const file of ['LICENSE', 'THIRD_PARTY_NOTICES.md', 'TYPST-ASSETS-LICENSE', 'TYPST-ASSETS-NOTICE']) {
  copyFile(path.join(inputs[0].bundle, file), path.join(directory, file));
}
for (const input of inputs) {
  const [os] = platforms[input.target];
  const native = path.join(directory, 'native', platforms[input.target].join('-'));
  fs.mkdirSync(native, { recursive: true });
  const executable = os === 'win32' ? 'canisend.exe' : 'canisend';
  for (const file of [executable, 'TARGET', 'RELEASE.json']) {
    copyFile(path.join(input.bundle, file), path.join(native, file));
  }
  fs.chmodSync(path.join(native, executable), 0o755);
}
copyFile(fileURLToPath(new URL('canisend.cjs', import.meta.url)), path.join(directory, 'canisend.cjs'));
const sourceArchive = path.join(inputs[0].bundle, 'SOURCE.tar.gz');
if (fs.existsSync(sourceArchive)) copyFile(sourceArchive, path.join(directory, 'SOURCE.tar.gz'));
fs.writeFileSync(path.join(directory, 'package.json'), JSON.stringify({
  name: 'canisend', version, license: 'GPL-3.0-only',
  repository: { type: 'git', url: 'git+https://github.com/jxpeng98/CanISend.git' },
  description: 'Evidence-bound application preparation CLI', bin: { canisend: 'canisend.cjs' }, engines: { node: '>=22.14' },
  ...(inputs.length === 1 ? platformMetadata(inputs[0].target) : {}),
}, null, 2) + '\n');
fs.writeFileSync(path.join(directory, 'README.md'), `# CanISend CLI

Prepare evidence-bound applications locally. This single npm package includes the Rust CLI.

## Install

\`\`\`sh
npm install -g canisend@${version}
canisend --workspace ./applications workspace init --host codex
\`\`\`

Requires Node.js 22.14 or newer. No platform packages, install scripts, or additional binary downloads are needed.

This package version supports:
${inputs.map(input => `- ${input.target}`).join('\n')}

The CLI embeds both workflow Packs and all five Skills. Workspace initialization with \`--host codex\` installs the Skills in \`.agents/skills\`.

Application evidence, consent, review, and export controls remain active. CanISend does not upload or submit applications.

[Source and documentation](https://github.com/jxpeng98/CanISend)
${fs.existsSync(sourceArchive) ? '\nThe corresponding source is included in `SOURCE.tar.gz`. Extract it and run `cargo build --release --locked -p canisend` to build the CLI.\n' : ''}`);
const npm = process.platform === 'win32' ? 'npm.cmd' : 'npm';
const packed = JSON.parse(execFileSync(npm, ['pack', '--ignore-scripts', '--json', '--pack-destination', output], { cwd: directory, encoding: 'utf8' }));
const archives = [packed[0].filename];
fs.writeFileSync(path.join(output, 'packages.json'), JSON.stringify({ version, complete: seen.size === Object.keys(platforms).length, archives }, null, 2) + '\n');
