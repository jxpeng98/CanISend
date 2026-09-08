#!/usr/bin/env python3
"""Vendor two local ModernPro releases; run the Rust source gate after syncing.

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
    declarations_path = RESOURCES / "manifest.json"
    declarations = json.loads(declarations_path.read_bytes())
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
    files.update({declarations_path: catalog_bytes(declarations_path, declarations),
                  pack_path: catalog_bytes(pack_path, pack),
                  history_path: encoded(history), contract_path: encoded(contract),
                  ROOT / "release/modernpro-sources.json": encoded(pins)})
    package_path = ROOT / "release/alpha-package-contract.json"
    package = json.loads(package_path.read_bytes())
    package["contracts"]["resource_manifest"]["sha256"] = sha(files[declarations_path])
    for binding in package["contracts"]["workflow_packs"]:
        if binding["id"] == pack["id"]:
            binding.update(version=pack["version"], content_digest=pack["content_digest"])
    files[package_path] = encoded(package)
    drift = [path for path, body in files.items() if path.read_bytes() != body]
    if check and drift:
        raise ValueError("template synchronization required: " + ", ".join(str(p.relative_to(ROOT)) for p in drift))
    if not check:
        for path in drift:
            path.write_bytes(files[path])
    print(f"Typst templates: {'verified' if check else 'synchronized'}; academic Pack {pack['version']}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("cv", type=Path)
    parser.add_argument("coverletter", type=Path)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    synchronize(args.cv, args.coverletter, args.check)
