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

CanISend helps you prepare applications using your own sources and evidence. It keeps requirements, document drafts, reviews, and exports in a local Workspace so you can return to an application without losing its history.

The CLI includes two workflow Packs: a general application Pack and an academic-job Pack. You can use both in the same Workspace. CanISend prepares materials for you to review and submit yourself; it does not log in to application sites, upload documents, or submit applications.

## Install

Install this version and check the command your shell runs:

\`\`\`sh
npm install -g canisend@${version}
canisend version
\`\`\`

You need Node.js 22.14 or newer for the npm launcher. The CLI and MCP server are compiled Rust programs; no Rust toolchain is needed for this installation. This single package contains the native executables under \`native/\`, with no platform-package dependencies, install scripts, or extra binary downloads.

This package contains binaries for:

${inputs.map(input => `- ${input.target}`).join('\n')}

## Create a Workspace and connect Codex

Choose a private directory for your applications:

\`\`\`sh
canisend --workspace ./applications workspace init --host codex
canisend --workspace ./applications workspace check
\`\`\`

Initialization creates the Workspace and installs five Skills in \`./applications/.agents/skills\`. Open that Workspace in Codex so it can discover the project Skills.

| Skill | What it helps with |
| --- | --- |
| \`canisend-application-workflow\` | Start or resume an application and coordinate its stages. |
| \`canisend-workspace\` | Set up the Workspace, manage shared data, and check or recover its state. |
| \`canisend-intake\` | Read source material and extract, correct, and confirm requirements. |
| \`canisend-materials\` | Assess fit, plan documents, draft, and revise using supported facts. |
| \`canisend-review-export\` | Review materials, resolve findings, render, and export local files. |

Initialization prints an MCP registration command for your installation. Run that command, then reconnect Codex and check that the CanISend tools are available. Installing Skills alone does not register or verify the MCP connection.

The registration command points directly to the Rust executable. With npm, its path includes \`node_modules/canisend/native/...\`; \`node_modules\` is the installation directory, and the MCP server itself runs without Node. \`Agent v4\` identifies the integration protocol, separately from the package version shown by \`canisend version\`.

For Claude Code, use \`--host claude\`. Use \`--no-skills\` to initialize without Skills, or add \`--scope global\` with a host to install them for your user account. For an existing Workspace, use \`workspace upgrade\` to refresh installed project Skills, or add \`--host codex\` to install them.

## Prepare an application

In your connected host, ask CanISend to help with an opportunity and provide the relevant sources. The workflow moves through requirements, evidence, a document plan, drafts, review, and local export. Claims need supporting evidence; you retain control over required approvals and the final submission.

Workspace data stays local. If you use an external AI host, the content you share with it is subject to that host's settings and data policy. Keep private documents, Workspaces, and credentials out of public issue reports.

## Upgrade

Back up your Workspace to a new directory before upgrading:

\`\`\`sh
canisend --workspace ./applications workspace backup ./applications-backup
npm install -g canisend@next
canisend version
canisend --workspace ./applications workspace upgrade
canisend --workspace ./applications workspace check
\`\`\`

\`next\` tracks testing releases and may differ from npm's default \`latest\` tag. To choose a specific release, install \`canisend@VERSION\` instead.

Review the MCP registration printed by \`host setup\`, update the host registration, and reconnect. The executable path changed between the beta.3 platform packages and the beta.4 single package, so an older registration can still point to the previous binary. Refresh project Skills through \`workspace upgrade\`; it reports conflicts rather than silently replacing locally edited files. Keep the backup until you have checked the upgraded Workspace.

## If the command still shows an older version

On macOS or Linux, compare the command path with npm's installation directory:

\`\`\`sh
type -a canisend
npm prefix -g
npm ls -g canisend --depth=0
\`\`\`

On Windows, use \`where.exe canisend\` to list matching commands. An older executable, shell alias, or version-manager shim may take precedence over npm's command. Check your PATH and npm prefix, then update the conflicting entry. Reinstalling the package alone will not change which command your shell selects.

## Source, license, and help

CanISend is licensed under GPL-3.0-only. The package includes the license and third-party notices.

[Source and documentation](https://github.com/jxpeng98/CanISend) · [Release scope and limitations](https://github.com/jxpeng98/CanISend/blob/main/RELEASE.md) · [Report an issue](https://github.com/jxpeng98/CanISend/issues)
${fs.existsSync(sourceArchive) ? '\nThe corresponding source is included in `SOURCE.tar.gz`. Extract it and run `cargo build --release --locked -p canisend` with the pinned Rust toolchain to build the CLI. The resulting native executable runs without Node.js.\n' : ''}`);
const npm = process.platform === 'win32' ? 'npm.cmd' : 'npm';
const packed = JSON.parse(execFileSync(npm, ['pack', '--ignore-scripts', '--json', '--pack-destination', output], { cwd: directory, encoding: 'utf8' }));
const archives = [packed[0].filename];
fs.writeFileSync(path.join(output, 'packages.json'), JSON.stringify({ version, complete: seen.size === Object.keys(platforms).length, archives }, null, 2) + '\n');
