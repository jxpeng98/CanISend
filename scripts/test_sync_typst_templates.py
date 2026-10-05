"""Run with python3 -m unittest discover -s scripts -p test_sync_typst_templates.py."""

import json
from pathlib import Path
import shutil
import tempfile
import unittest
from unittest.mock import patch

import sync_typst_templates as sync


def copy_fixture(root):
    for relative in ["crates/canisend-resources", "release"]:
        shutil.copytree(sync.ROOT / relative, root / relative)
    shutil.copyfile(sync.ROOT / "AGENTS.md", root / "AGENTS.md")
    guide = root / "docs/guides/typst-templates.md"
    guide.parent.mkdir(parents=True)
    shutil.copyfile(sync.ROOT / "docs/guides/typst-templates.md", guide)


class TemplateSyncTest(unittest.TestCase):
    def test_upgrade_preserves_history_is_idempotent_and_rejects_invalid_input(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            copy_fixture(root)
            pins = json.loads((root / "release/modernpro-sources.json").read_bytes())
            resources = root / "crates/canisend-resources/resources"
            inputs = []
            for name, pin in pins.items():
                directory = root / name
                directory.mkdir()
                major, minor, patch_version = map(int, pin["version"].split("."))
                version = f"{major}.{minor}.{patch_version + 1}"
                (directory / "typst.toml").write_text(
                    f'[package]\nname = "{name}"\nversion = "{version}"\n'
                    f'entrypoint = "{name}.typ"\nlicense = "MIT"\n')
                body = (resources / f"templates/{name}.typ").read_bytes()[:pin["source_bytes"]]
                (directory / f"{name}.typ").write_bytes(body + b"// synthetic upgrade\n")
                inputs.append(directory)
            inputs.sort(key=lambda path: path.name != "modernpro-cv")
            history_path = root / "crates/canisend-resources/history/academic-job.json"
            history = json.loads(history_path.read_bytes())
            pack_path = resources / "workflow-packs/org.canisend.academic-job/manifest.json"
            old_manifest = pack_path.read_text()
            package_path = root / "release/alpha-package-contract.json"
            package = json.loads(package_path.read_bytes())
            package["contracts"]["resource_manifest"]["entry_count"] = 0
            package_path.write_bytes(sync.encoded(package))
            with patch.object(sync, "ROOT", root), patch.object(sync, "RESOURCES", resources), \
                    patch.object(sync.subprocess, "check_output", return_value="a" * 40):
                with self.assertRaises(ValueError):
                    sync.synchronize(*inputs, check=True)
                self.assertEqual(pack_path.read_text(), old_manifest)
                sync.synchronize(*inputs)
                self.assertEqual(json.loads(package_path.read_bytes())["contracts"]["resource_manifest"]["entry_count"],
                                 len(json.loads((resources / "manifest.json").read_bytes())))
                upgraded = json.loads(history_path.read_bytes())
                self.assertEqual(upgraded[:-1], history)
                self.assertEqual(upgraded[-1]["manifest"], old_manifest)
                declarations = json.loads((resources / "manifest.json").read_bytes())
                for item in declarations:
                    if item["id"] in sync.GUIDANCE_RESOURCES:
                        guide = (resources / item["path"]).read_text()
                        for directory in inputs:
                            version = sync.tomllib.loads((directory / "typst.toml").read_text())["package"]["version"]
                            self.assertIn(f"@preview/{directory.name}:{version}", guide)
                            self.assertNotIn(f"@preview/{directory.name}:{pins[directory.name]['version']}", guide)
                for path in [root / "AGENTS.md", root / "docs/guides/typst-templates.md"]:
                    for directory in inputs:
                        version = sync.tomllib.loads((directory / "typst.toml").read_text())["package"]["version"]
                        self.assertIn(f"@preview/{directory.name}:{version}", path.read_text())
                snapshot = {path: path.read_bytes() for path in root.rglob("*") if path.is_file()}
                sync.synchronize(*inputs, check=True)
                sync.synchronize(*inputs)
                self.assertTrue(all(path.read_bytes() == body for path, body in snapshot.items()))
                (inputs[1] / "typst.toml").write_text('[package]\nname = "wrong"\nversion = "0.0.0"\n')
                with self.assertRaises(ValueError):
                    sync.synchronize(*inputs)
                self.assertEqual(pack_path.read_bytes(), snapshot[pack_path])
                self.assertEqual(history_path.read_bytes(), snapshot[history_path])
                damaged = resources / "templates/modernpro-cv.typ"
                damaged.write_bytes(damaged.read_bytes() + b"// unrecorded edit\n")
                with self.assertRaisesRegex(ValueError, "current Pack resource was modified"):
                    sync.synchronize(*inputs)
                self.assertEqual(history_path.read_bytes(), snapshot[history_path])

    def test_guidance_refresh_repairs_drift_without_changing_templates_or_pack_history(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            copy_fixture(root)
            resources = root / "crates/canisend-resources/resources"
            declarations_path = resources / "manifest.json"
            declarations = json.loads(declarations_path.read_bytes())
            before = {path: path.read_bytes() for path in root.rglob("*") if path.is_file()}
            guide = resources / "agent/codex/AGENTS.md"
            cv_version = json.loads((root / "release/modernpro-sources.json").read_bytes())["modernpro-cv"]["version"]
            guide.write_text(guide.read_text().replace(f"@preview/modernpro-cv:{cv_version}", "@preview/modernpro-cv:0.0.0"))
            with patch.object(sync, "ROOT", root), patch.object(sync, "RESOURCES", resources):
                with self.assertRaisesRegex(ValueError, "template synchronization required"):
                    sync.refresh_guidance(check=True)
                self.assertIn("@preview/modernpro-cv:0.0.0", guide.read_text())
                sync.refresh_guidance()
                self.assertEqual(guide.read_bytes(), before[guide])
                updated = json.loads(declarations_path.read_bytes())
                for old, new in zip(declarations, updated, strict=True):
                    if old["id"] == "agent.codex.guide":
                        major, minor, patch_version = map(int, old["version"].split("."))
                        self.assertEqual(new["version"], f"{major}.{minor}.{patch_version + 1}")
                    else:
                        self.assertEqual(old, new)
                for path, body in before.items():
                    if path not in [declarations_path, root / "release/alpha-package-contract.json"]:
                        self.assertEqual(path.read_bytes(), body)
                snapshot = {path: path.read_bytes() for path in root.rglob("*") if path.is_file()}
                sync.refresh_guidance(check=True)
                sync.refresh_guidance()
                self.assertTrue(all(path.read_bytes() == body for path, body in snapshot.items()))

    def test_guidance_rejects_bad_pins_or_missing_markers_before_any_write(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            copy_fixture(root)
            resources = root / "crates/canisend-resources/resources"
            pins_path = root / "release/modernpro-sources.json"
            original = pins_path.read_bytes()
            pins = json.loads(original)
            pins["modernpro-cv"]["version"] = "0.0.0"
            pins_path.write_bytes(sync.encoded(pins))
            with patch.object(sync, "ROOT", root), patch.object(sync, "RESOURCES", resources):
                before = {path: path.read_bytes() for path in root.rglob("*") if path.is_file()}
                with self.assertRaisesRegex(ValueError, "pin differs"):
                    sync.refresh_guidance()
                self.assertTrue(all(path.read_bytes() == body for path, body in before.items()))
                pins_path.write_bytes(original)
                guide = root / "AGENTS.md"
                guide.write_text(guide.read_text().replace(sync.GUIDANCE_END, ""))
                before = {path: path.read_bytes() for path in root.rglob("*") if path.is_file()}
                with self.assertRaisesRegex(ValueError, "guidance markers"):
                    sync.refresh_guidance()
                self.assertTrue(all(path.read_bytes() == body for path, body in before.items()))
