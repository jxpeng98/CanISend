#!/usr/bin/env python3
"""Sync ModernPro bundles and pinned Preview guidance; run the Rust source gate afterwards.

Python 3.11+. No network access. Historical Pack bindings remain embedded.
"""

import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import tomllib


ROOT = Path(__file__).resolve().parents[1]
RESOURCES = ROOT / "crates/canisend-resources/resources"
MARKER = "// CanISend offline adapter."
GUIDANCE_START = "<!-- canisend:typst-preview:start -->"
GUIDANCE_END = "<!-- canisend:typst-preview:end -->"
HOST_GUIDES = {
    "agent.codex.guide": ("codex", ".agents/skills"),
    "agent.claude.guide": ("claude", ".claude/skills"),
    "agent.generic.guide": ("generic", "skills"),
}
GUIDANCE_RESOURCES = {
    *HOST_GUIDES,
    "skill.canisend-materials",
}


def agent_guidance(host, skill_root):
    return f'''## Authoring guidance and upgrades

For CanISend authoring, read the installed [canisend-materials Skill]({skill_root}/canisend-materials/SKILL.md).
Its template guidance supplies the exact Typst Preview package versions, entry points and
CV, cover-letter and statement examples. Load it again after a CLI/Workspace upgrade or Host
reconnection; use its current versions rather than remembered imports or copied Agent examples.
If the user chose global Skills, read the Materials Skill from that installation instead.

After backing up the Workspace and installing the new CLI, run
`canisend --workspace PATH workspace upgrade` to refresh existing project Skills.
If this Host's Skills are absent, use `canisend --workspace PATH workspace upgrade --host {host}`
to install them. Global Skills use `canisend --workspace PATH host setup --host {host} --scope global`.
Check `canisend --workspace PATH host status --host {host}` (with `--scope global` for global Skills)
and reconnect the Host before continuing. If managed files were edited or are unmanaged,
preserve the customizations and resolve the reported conflict; never force-update them.

For user-owned Typst documents, keep exact imports and review any version change by compiling
and inspecting the affected PDFs. No template repository checkout is needed. CanISend-managed
Deliverables retain their exact bound Pack templates, structured review and guarded export;
the embedded renderer works offline and does not compile arbitrary Preview imports.

This Agent guide is a durable pointer, not a copy of the template pins. Setup and Workspace
upgrade preserve user-owned `AGENTS.md` and `CLAUDE.md`. When adopting this guide, review its
merge into user guidance. An exported Agent pack's bundled resources remain snapshots;
use the selected Workspace/Host's managed Skill installation for ongoing work. Regenerate the
pack when its protocol or workflow instructions change.
'''


def preview_guidance(pins):
    cv = pins["modernpro-cv"]["version"]
    letter = pins["modernpro-coverletter"]["version"]
    return f'''## Typst Preview templates

For user-owned standalone Typst files, import these exact published packages. No template
repository checkout or copied package implementation is needed. Keep the full version in every
import; change it only during a reviewed upgrade, then compile and inspect the affected PDFs.

| Material | Package import | Entry point |
| --- | --- | --- |
| Academic CV | `@preview/modernpro-cv:{cv}` | `cv` |
| Cover letter | `@preview/modernpro-coverletter:{letter}` | `coverletter` |
| Research or teaching statement | `@preview/modernpro-coverletter:{letter}` | `statement` |

Define the same reviewed `profile` values inline in each document. These fictional values are
syntax examples, not applicant facts; replace them with user-reviewed information:

```typst
#let profile = (
  name: [Fictional Candidate],
  contacts: ((text: [candidate\\@example.invalid], link: "mailto:candidate@example.invalid"),),
)
```

Use one of the following snippets in each file, after its profile definition. For a CV, prefer
one column for academic applications and reliable PDF reading order:

```typst
#import "@preview/modernpro-cv:{cv}": cv, section, summary
#show: cv.with(profile: profile, theme: (font: "Libertinus Serif"), options: (last-updated: false))
#section("Research Profile")
#summary[Replace with a reviewed research summary.]
```

For a cover letter, supply reviewed recipient values and write short paragraphs:

```typst
#import "@preview/modernpro-coverletter:{letter}": coverletter
#show: coverletter.with(
  profile: profile,
  recipient: (organization: [Fictional Institution], greeting: [Dear Search Committee,]),
  theme: (font: "Libertinus Serif"),
)
Replace with reviewed letter text.
```

Use the same package for each statement; set its title and headings to the actual requirements:

```typst
#import "@preview/modernpro-coverletter:{letter}": statement
#show: statement.with(profile: profile, title: [Research Statement], theme: (font: "Libertinus Serif"))
= Research agenda
Replace with reviewed statement text.
```

Compile a user-owned file with `typst compile cv.typ cv.pdf` (or the corresponding letter or
statement paths) in the external Typst CLI. Its first import of a version requires a package
download; cached packages work offline. Ensure the selected font is available to that compiler.
Escape Typst syntax in user text, including `\\@` in content blocks; omit optional icons and
photos unless requested. Inspect contacts, page breaks and extracted PDF text before delivery.

CanISend-managed Deliverables use the verified template in their exact bound Workflow Pack,
through structured content, review and guarded export. The embedded renderer works offline and
does not compile arbitrary Preview imports or user-authored Typst. A standalone PDF is not a
CanISend-reviewed export. Do not replace managed projections with these examples or treat template
placeholders as Evidence. Existing Applications retain their original Pack/template identity.

After upgrading the CLI, run `canisend --workspace PATH workspace upgrade` to refresh project
Skills, including this template guidance; global Skills use `host setup --host HOST --scope global`.
Reconnect the Host and reread the installed Materials Skill for current pins and APIs. Bundled
Host guides delegate authoring details to this Skill; a durable pointer in user-owned `AGENTS.md`
does not need new version numbers on each template upgrade. Exported Agent packs still contain
resource snapshots; use the selected Workspace/Host's managed Skills for ongoing work, and
regenerate a pack when its protocol or workflow instructions change. Never overwrite a user's
`AGENTS.md` or `CLAUDE.md`.

API references: [CV](https://typst.app/universe/package/modernpro-cv/),
[letters and statements](https://typst.app/universe/package/modernpro-coverletter/),
[package versions and caching](https://github.com/typst/packages/blob/main/README.md).
'''


def replace_guidance(path, block):
    body = path.read_text()
    if body.count(GUIDANCE_START) != 1 or body.count(GUIDANCE_END) != 1:
        raise ValueError(f"missing or duplicate Preview guidance markers: {path}")
    before, rest = body.split(GUIDANCE_START)
    _, after = rest.split(GUIDANCE_END)
    return (before + GUIDANCE_START + "\n" + block + GUIDANCE_END + after).encode()


def guidance_files(pins, declarations):
    declared_guides = {item["id"] for item in declarations} & GUIDANCE_RESOURCES
    if declared_guides != GUIDANCE_RESOURCES:
        raise ValueError("missing Preview guidance resource declarations")
    for name in ["modernpro-cv", "modernpro-coverletter"]:
        pin = pins[name]
        declaration = next(item for item in declarations if item["id"] == "template." + name)
        if (pin["package"] != name or pin["version"] != declaration["version"]
                or not re.fullmatch(r"\d+\.\d+\.\d+", pin["version"])):
            raise ValueError(f"Preview guidance pin differs from the template: {name}")
    block = preview_guidance(pins)
    files = {}
    for declaration in declarations:
        if declaration["id"] in GUIDANCE_RESOURCES:
            path = RESOURCES / declaration["path"]
            guide = HOST_GUIDES.get(declaration["id"])
            body = replace_guidance(path, agent_guidance(*guide) if guide else block)
            files[path] = body
            if body != path.read_bytes():
                major, minor, patch = map(int, declaration["version"].split("."))
                declaration["version"] = f"{major}.{minor}.{patch + 1}"
    files[ROOT / "docs/guides/typst-templates.md"] = replace_guidance(
        ROOT / "docs/guides/typst-templates.md", block)
    cv = pins["modernpro-cv"]["version"]
    letter = pins["modernpro-coverletter"]["version"]
    files[ROOT / "AGENTS.md"] = replace_guidance(ROOT / "AGENTS.md", f'''## Typst templates

Standalone authoring imports `@preview/modernpro-cv:{cv}` for CVs and
`@preview/modernpro-coverletter:{letter}` for cover letters and statements. See the
[template guide](docs/guides/typst-templates.md) for APIs and upgrade instructions.
Keep these pins aligned with `release/modernpro-sources.json` and the embedded template contract.
`scripts/sync_typst_templates.py` updates this section, shipped Host guides and Materials Skills
alongside template changes. Use `--guidance-only --check` to check guidance without source checkouts.
Managed rendering retains its offline, exact-Pack boundary; user-owned Preview documents use
the external Typst compiler.
''')
    return files


def bind_resource_manifest(files, declarations):
    path = RESOURCES / "manifest.json"
    files[path] = catalog_bytes(path, declarations)
    package_path = ROOT / "release/alpha-package-contract.json"
    package = json.loads(files.get(package_path, package_path.read_bytes()))
    package["contracts"]["resource_manifest"]["sha256"] = sha(files[path])
    package["contracts"]["resource_manifest"]["entry_count"] = len(declarations)
    files[package_path] = encoded(package)


def apply_files(files, check):
    drift = [path for path, body in files.items() if path.read_bytes() != body]
    if check and drift:
        raise ValueError("template synchronization required: " + ", ".join(str(p.relative_to(ROOT)) for p in drift))
    if not check:
        for path in drift:
            path.write_bytes(files[path])


def refresh_guidance(check=False):
    pins = json.loads((ROOT / "release/modernpro-sources.json").read_bytes())
    declarations = json.loads((RESOURCES / "manifest.json").read_bytes())
    files = guidance_files(pins, declarations)
    bind_resource_manifest(files, declarations)
    apply_files(files, check)
    print(f"Typst Preview guidance: {'verified' if check else 'synchronized'}")


def encoded(value):
    return (json.dumps(value, ensure_ascii=False, indent=2) + "\n").encode()


def sha(data):
    return hashlib.sha256(data).hexdigest()


def catalog_bytes(path, value):
    """Keep the existing hand-formatted catalogs reviewable."""
    text = path.read_text()
    before = json.loads(text)
    rows = value if isinstance(value, list) else value["resources"]
    old_rows = before if isinstance(before, list) else before["resources"]
    for old, new in zip(old_rows, rows, strict=True):
        if old != new:
            for separators in [(",", ":"), (", ", ": ")]:
                text = text.replace(json.dumps(old, separators=separators),
                                    json.dumps(new, separators=separators))
    if isinstance(value, dict):
        text = text.replace(f'"version": "{before["version"]}"',
                            f'"version": "{value["version"]}"', 1)
        text = text.replace(before["content_digest"], value["content_digest"])
    if json.loads(text) != value:
        raise ValueError(f"unsupported catalog layout: {path}")
    return text.encode()


def synchronize(cv, letter, check=False):
    declarations = json.loads((RESOURCES / "manifest.json").read_bytes())
    pack_path = RESOURCES / "workflow-packs/org.canisend.academic-job/manifest.json"
    pack = json.loads(pack_path.read_bytes())
    history_path = ROOT / "crates/canisend-resources/history/academic-job.json"
    history = json.loads(history_path.read_bytes())
    old = {
        "manifest": pack_path.read_bytes().decode(),
        "resources": {item["path"]: (RESOURCES / item["path"]).read_bytes().decode()
                      for item in pack["resources"]},
    }
    for item in pack["resources"]:
        body = old["resources"][item["path"]].encode()
        if len(body) != item["size_bytes"] or sha(body) != item["sha256"]:
            raise ValueError(f"current Pack resource was modified: {item['path']}")
    contract_path = ROOT / "release/typst-template-contract.json"
    contract = json.loads(contract_path.read_bytes())
    pins = {}
    files = {}
    changed = False
    for name, directory in [("modernpro-cv", cv), ("modernpro-coverletter", letter)]:
        package = tomllib.loads((directory / "typst.toml").read_text())["package"]
        version = package["version"]
        if (package["name"] != name or package["entrypoint"] != name + ".typ"
                or package["license"] != "MIT" or not re.fullmatch(r"\d+\.\d+\.\d+", version)):
            raise ValueError(f"unexpected package identity: {name}")
        source = (directory / package["entrypoint"]).read_bytes()
        path = RESOURCES / "templates" / (name + ".typ")
        previous = path.read_bytes().decode()
        adapter = MARKER + previous.split(MARKER, 1)[1]
        adapter = re.sub(r"@preview/" + name + r":\d+\.\d+\.\d+", f"@preview/{name}:{version}", adapter)
        body = source + (b"" if source.endswith(b"\n") else b"\n") + adapter.encode()
        changed |= body != path.read_bytes()
        files[path] = body
        pin = {
            "license": "MIT", "package": name,
            "repository": "https://github.com/jxpeng98/" + ("Typst-CV-Resume" if name == "modernpro-cv" else "typst-coverletter"),
            "source_entrypoint": package["entrypoint"],
            "source_commit": subprocess.check_output(["git", "-C", str(directory), "rev-parse", "HEAD"], text=True).strip(),
            "source_sha256": sha(source), "source_bytes": len(source),
            "source_patches": [], "version": version,
        }
        pins[name] = pin
        for item in declarations:
            if item["id"] == "template." + name:
                item["version"] = version
        for item in pack["resources"]:
            if item["id"] == name:
                item.update(version=version, size_bytes=len(body), sha256=sha(body))
        for item in contract["templates"]:
            if item["resource_id"] == "template." + name:
                item.update(resource_version=version, bytes=len(body), sha256=sha(body), upstream=pin)

    if changed:
        if any(json.loads(item["manifest"])["version"] == pack["version"] for item in history):
            raise ValueError("current Pack version already archived; refusing to replace history")
        history.append(old)
        major, minor, patch = map(int, pack["version"].split("."))
        pack["version"] = f"{major}.{minor}.{patch + 1}"
        pack["content_digest"] = "0" * 64
        # This is the workflow-pack/v1 wire digest; the Rust loader validates it.
        digest = hashlib.sha256(b"canisend.workflow-pack-bundle/v1\0")
        def segment(data):
            digest.update(len(data).to_bytes(8, "big"))
            digest.update(data)
        segment(b"manifest")
        segment(json.dumps(pack, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode())
        for item in sorted(pack["resources"], key=lambda item: item["path"]):
            path = RESOURCES / item["path"]
            segment(b"resource-path")
            segment(item["path"].encode())
            segment(b"resource-bytes")
            segment(files.get(path, path.read_bytes()))
        pack["content_digest"] = digest.hexdigest()
    for item in declarations:
        if item["path"] == str(pack_path.relative_to(RESOURCES)):
            item["version"] = pack["version"]
    contract["baseline"] = "modernpro-source-pinned-v3"
    files.update(guidance_files(pins, declarations))
    files.update({pack_path: catalog_bytes(pack_path, pack),
                  history_path: encoded(history), contract_path: encoded(contract),
                  ROOT / "release/modernpro-sources.json": encoded(pins)})
    package_path = ROOT / "release/alpha-package-contract.json"
    package = json.loads(package_path.read_bytes())
    for binding in package["contracts"]["workflow_packs"]:
        if binding["id"] == pack["id"]:
            binding.update(version=pack["version"], content_digest=pack["content_digest"])
    files[package_path] = encoded(package)
    bind_resource_manifest(files, declarations)
    apply_files(files, check)
    print(f"Typst templates: {'verified' if check else 'synchronized'}; academic Pack {pack['version']}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("cv", type=Path, nargs="?")
    parser.add_argument("coverletter", type=Path, nargs="?")
    parser.add_argument("--guidance-only", action="store_true",
                        help="refresh Preview guidance from committed pins without template source checkouts")
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    if args.guidance_only:
        if args.cv or args.coverletter:
            parser.error("--guidance-only does not take source directories")
        refresh_guidance(args.check)
    else:
        if not args.cv or not args.coverletter:
            parser.error("provide both template source directories or --guidance-only")
        synchronize(args.cv, args.coverletter, args.check)
