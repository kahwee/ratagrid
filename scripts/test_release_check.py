"""Release guard regressions: refuse dirty, mismatched or altered artifacts."""
import hashlib
import os
import subprocess
import sys
import textwrap
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


class PublishingBoundaryTests(unittest.TestCase):
    def workflow(self):
        return (release.ROOT / '.github/workflows/publish-crate.yml').read_text()

    def test_builds_and_repository_scripts_have_no_oidc(self):
        verify, publish = self.workflow().split('  publish:\n', 1)
        self.assertNotIn('id-token: write', verify)
        self.assertIn('needs: verify', publish)
        self.assertIn('id-token: write', publish)
        self.assertNotIn('actions/checkout', publish)
        self.assertNotIn('scripts/', publish)
        self.assertIn('cargo +1.99.1 publish --locked --no-verify --registry crates-io', publish)
        self.assertIn('CARGO_HOME: ${{ runner.temp }}/publish-cargo-home', publish)

    def test_artifact_validation_rejects_links_paths_and_digest_mismatch(self):
        workflow = self.workflow()
        code = textwrap.dedent(workflow.split("python3 -I - <<'PYTHON'\n", 1)[1].split('          PYTHON', 1)[0])
        for case in ('valid', 'symlink', 'traversal', 'digest', 'sha', 'tag'):
            with self.subTest(case=case), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                artifact = root / 'verified-crate'
                artifact.mkdir()
                archive = artifact / 'ratagrid-1.2.3.crate'
                with tarfile.open(archive, 'w:gz') as package:
                    name = 'ratagrid-1.2.3/Cargo.toml'
                    if case == 'traversal':
                        name = 'ratagrid-1.2.3/../../escaped'
                    entry = tarfile.TarInfo(name)
                    if case == 'symlink':
                        entry.type = tarfile.SYMTYPE
                        entry.linkname = '/tmp/other'
                        package.addfile(entry)
                    else:
                        entry.size = len(MANIFEST)
                        package.addfile(entry, io.BytesIO(MANIFEST))
                        original = tarfile.TarInfo('ratagrid-1.2.3/Cargo.toml.orig')
                        original.size = len(MANIFEST)
                        package.addfile(original, io.BytesIO(MANIFEST))
                evidence = {'version': VERSION, 'sha': SHA,
                            'sha256': hashlib.sha256(archive.read_bytes()).hexdigest()}
                if case == 'digest':
                    evidence['sha256'] = '0' * 64
                if case == 'sha':
                    evidence['sha'] = 'b' * 40
                (artifact / 'release-preflight.json').write_text(json.dumps(evidence))
                env = dict(os.environ, RUNNER_TEMP=temporary, SOURCE_SHA=SHA,
                           RELEASE_TAG='v9.9.9' if case == 'tag' else 'v' + VERSION)
                result = subprocess.run([sys.executable, '-I', '-'], input=code,
                                        text=True, capture_output=True, env=env)
                self.assertEqual(result.returncode == 0, case == 'valid', result.stderr)
                if case == 'valid':
                    self.assertEqual((root / 'crate-publish/ratagrid-1.2.3/Cargo.toml').read_bytes(), MANIFEST)
                    self.assertFalse((root / 'crate-publish/ratagrid-1.2.3/Cargo.toml.orig').exists())


if __name__ == '__main__':
    unittest.main()
