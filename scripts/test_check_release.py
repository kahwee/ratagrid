import importlib.util
from pathlib import Path
import unittest
import subprocess
import sys

spec = importlib.util.spec_from_file_location("check", Path(__file__).with_name("check-release.py"))
check = importlib.util.module_from_spec(spec)
spec.loader.exec_module(check)


class MetadataTests(unittest.TestCase):
    def test_cli_validates_actual_checked_out_metadata_and_rejects_bad_tag(self):
        directory = Path(__file__).resolve().parent.parent
        manifest = check.tomllib.loads((directory / "Cargo.toml").read_text())
        script = directory / "scripts/check-release.py"
        version = manifest["package"]["version"]
        self.assertEqual(subprocess.run([sys.executable, str(script), "v" + version], cwd=directory, capture_output=True).returncode, 0)
        self.assertEqual(subprocess.run([sys.executable, str(script), "v0.0.0-invalid"], cwd=directory, capture_output=True).returncode, 1)

    def test_manifest_tag_and_lock_must_match(self):
        manifest = {"package": {"name": "example", "version": "1.2.3"}}
        lock = {"package": [{"name": "example", "version": "1.2.3"}]}
        self.assertEqual(check.validate(manifest, lock, "v1.2.3"), ("example", "1.2.3"))
        with self.assertRaises(ValueError):
            check.validate(manifest, lock, "v1.2.4")
        with self.assertRaises(ValueError):
            check.validate(manifest, {"package": [{"name": "example", "version": "1.2.2"}]}, "v1.2.3")
        with self.assertRaises(ValueError):
            check.validate(manifest, {"package": []}, "v1.2.3")


if __name__ == "__main__":
    unittest.main()
