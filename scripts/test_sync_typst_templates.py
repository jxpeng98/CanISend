"""Run with python3 -m unittest discover -s scripts -p test_sync_typst_templates.py."""

import json
from pathlib import Path
import shutil
import tempfile
import unittest
from unittest.mock import patch

import sync_typst_templates as sync


class TemplateSyncTest(unittest.TestCase):
    def test_upgrade_preserves_history_is_idempotent_and_rejects_invalid_input(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for relative in ["crates/canisend-resources", "release"]:
                shutil.copytree(sync.ROOT / relative, root / relative)
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
            with patch.object(sync, "ROOT", root), patch.object(sync, "RESOURCES", resources), \
                    patch.object(sync.subprocess, "check_output", return_value="a" * 40):
                with self.assertRaises(ValueError):
                    sync.synchronize(*inputs, check=True)
                self.assertEqual(pack_path.read_text(), old_manifest)
                sync.synchronize(*inputs)
                upgraded = json.loads(history_path.read_bytes())
                self.assertEqual(upgraded[:-1], history)
                self.assertEqual(upgraded[-1]["manifest"], old_manifest)
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
