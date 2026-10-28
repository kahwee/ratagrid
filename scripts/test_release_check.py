"""Release guard regressions: refuse dirty, mismatched or altered artifacts."""
import importlib.util
import io
import json
from pathlib import Path
import tarfile
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('release_check', Path(__file__).with_name('release_check.py'))
release = importlib.util.module_from_spec(spec)
spec.loader.exec_module(release)

SHA = 'a' * 40
VERSION = '1.2.3'
MANIFEST = b'''[package]
name = "ratagrid"
version = "1.2.3"
license = "MIT"
edition = "2024"
rust-version = "1.99"
[dependencies]
ratatui-core = "0.1.2"
'''


class ReleaseChecks(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.archive = Path(self.directory.name) / 'ratagrid.crate'
        self.source = {'Cargo.toml': MANIFEST, 'Cargo.lock': b'[[package]]\nname = "ratagrid"\nversion = "1.2.3"\n',
                       'LICENSE': b'MIT fixture', 'src/lib.rs': b'pub fn example() {}',
                       'docs/rustdoc/features.md': b'features', 'docs/rustdoc/pagination.md': b'pagination',
                       'docs/rustdoc/updates.md': b'updates'}
        self.files = {**self.source, 'Cargo.toml.orig': MANIFEST,
                      '.cargo_vcs_info.json': json.dumps({'git': {'sha1': SHA}, 'path_in_vcs': ''}).encode()}

    def write(self, extra=()):
        with tarfile.open(self.archive, 'w:gz') as archive:
            for name, data in [*self.files.items(), *extra]:
                info = tarfile.TarInfo('ratagrid-1.2.3/' + name)
                info.size = len(data)
                archive.addfile(info, io.BytesIO(data))

    def inspect(self):
        return release.inspect_archive(self.archive, VERSION, SHA, self.source.__getitem__)

    def test_clean_source_and_existing_matching_tag_can_resume(self):
        release.validate_source('', SHA, None)
        release.validate_source('', SHA, SHA)
        for status, tag in [(' M src/lib.rs\n', None), ('?? extra.txt\n', None), ('', 'b' * 40)]:
            with self.subTest(status=status, tag=tag), self.assertRaises(ValueError):
                release.validate_source(status, SHA, tag)

    def test_publish_archive_location_prefers_staging_over_old_artifacts(self):
        target = Path(self.directory.name)
        with self.assertRaisesRegex(ValueError, 'did not produce'):
            release.package_archive(target, VERSION)
        legacy = target / 'package' / f'ratagrid-{VERSION}.crate'
        legacy.parent.mkdir()
        legacy.write_bytes(b'older artifact')
        self.assertEqual(release.package_archive(target, VERSION), legacy)
        staged = legacy.parent / 'tmp-registry' / legacy.name
        staged.parent.mkdir()
        staged.write_bytes(b'current publish artifact')
        self.assertEqual(release.package_archive(target, VERSION), staged)
        # An archive for another version must never be substituted.
        with self.assertRaisesRegex(ValueError, 'did not produce'):
            release.package_archive(target, '9.9.9')

    def test_valid_package_records_checksum_and_no_upload(self):
        self.write()
        report = self.inspect()
        self.assertEqual(report['sha'], SHA)
        self.assertEqual(report['sha256'], release.hashlib.sha256(self.archive.read_bytes()).hexdigest())
        self.assertFalse(report['registry_upload'])

    def test_foreign_or_dirty_source_metadata_is_rejected(self):
        for vcs in [{'git': {'sha1': 'b' * 40}, 'path_in_vcs': ''},
                    {'git': {'sha1': SHA, 'dirty': True}, 'path_in_vcs': ''},
                    {'git': {'sha1': SHA}, 'path_in_vcs': 'other'}]:
            self.files['.cargo_vcs_info.json'] = json.dumps(vcs).encode()
            self.write()
            with self.subTest(vcs=vcs), self.assertRaisesRegex(ValueError, 'VCS'):
                self.inspect()

    def test_source_change_or_missing_guide_is_rejected(self):
        self.files['src/lib.rs'] = b'altered source'
        self.write()
        with self.assertRaisesRegex(ValueError, 'differs from Git'):
            self.inspect()
        self.files['src/lib.rs'] = self.source['src/lib.rs']
        del self.files['docs/rustdoc/updates.md']
        self.write()
        with self.assertRaisesRegex(ValueError, 'missing required'):
            self.inspect()

    def test_development_files_duplicates_and_traversal_are_rejected(self):
        for name in ['AGENTS.md', '.github/workflows/ci.yml', 'docs/RELEASING.md',
                     'scripts/helper.py', 'src/lib.rs', '../outside', '/absolute']:
            self.write(extra=[(name, b'not package source')])
            with self.subTest(name=name), self.assertRaises(ValueError):
                self.inspect()

    def test_normalized_metadata_must_preserve_version_and_dependency_boundary(self):
        for original, replacement in [(b'1.2.3', b'1.2.4'),
                                      (b'license = "MIT"', b'license = "Apache-2.0"'),
                                      (b'ratatui-core', b'ratatui')]:
            self.files['Cargo.toml'] = MANIFEST.replace(original, replacement)
            self.write()
            with self.subTest(replacement=replacement), self.assertRaises(ValueError):
                self.inspect()


if __name__ == '__main__':
    unittest.main()
