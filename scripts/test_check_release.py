import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location("check", Path(__file__).with_name("check-release.py"))
check = importlib.util.module_from_spec(spec)
spec.loader.exec_module(check)


class MetadataTests(unittest.TestCase):
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
