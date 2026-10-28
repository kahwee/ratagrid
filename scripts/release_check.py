#!/usr/bin/env python3
"""Run local release checks or verify a clean package. Never uploads or tags."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path, PurePosixPath
import platform
import shlex
import subprocess
import sys
import tarfile
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parent.parent


def run(*args, cwd=None, env=None):
    print('+ ' + shlex.join(map(str, args)), flush=True)
    subprocess.run(args, cwd=ROOT if cwd is None else cwd, env=env, check=True)


def git(*args):
    return subprocess.check_output(['git', *args], cwd=ROOT)


def check():
    if platform.system() == 'Linux' and importlib.util.find_spec('pyte') is None:
        raise ValueError('Linux smoke tests need pyte==0.8.2 in this Python environment; see docs/RELEASING.md')
    run('cargo', 'fmt', '--all', '--check')
    run('cargo', 'clippy', '--all-targets', '--locked', '--', '-D', 'warnings')
    run('cargo', 'test', '--all-targets', '--locked')
    run('cargo', 'test', '--doc', '--locked')
    run('cargo', 'doc', '--no-deps', '--locked', env={**os.environ, 'RUSTDOCFLAGS': '-D warnings'})
    for script in ['test_check_release.py', 'test_release_check.py', 'test_benchmark_ci.py', 'test_build_docs.py']:
        run(sys.executable, str(ROOT / 'scripts' / script))
    run(sys.executable, str(ROOT / 'scripts/build_rustdoc_guides.py'), '--check')
    with tempfile.TemporaryDirectory() as outside:
        run(sys.executable, str(ROOT / 'scripts/build_docs.py'), '--check', cwd=outside)
    run('cargo', 'build', '--release', '--examples', '--locked')
    if platform.system() != 'Windows':
        run(sys.executable, str(ROOT / 'scripts/test_terminal_cleanup.py'))
    if platform.system() == 'Linux':
        run(sys.executable, str(ROOT / 'scripts/terminal_smoke.py'))
    print('Local checks passed. Remote CI must pass on the release commit too.')


def validate_source(status, sha, tag_sha):
    if status.strip():
        raise ValueError('Package verification requires a clean working tree; commit or isolate changes first')
    if tag_sha and tag_sha != sha:
        raise ValueError('This version already has a local tag on another commit; choose a new version, never move the tag')


def inspect_archive(archive, version, sha, source):
    """Compare packaged source to Git without extracting any archive entries."""
    prefix = f'ratagrid-{version}/'
    files = {}
    with tarfile.open(archive) as package:
        for member in package.getmembers():
            if member.isdir():
                continue
            if not member.isfile() or not member.name.startswith(prefix):
                raise ValueError(f'Unexpected package entry: {member.name}')
            name = member.name[len(prefix):]
            if '..' in PurePosixPath(name).parts or name.startswith('/') or name in files:
                raise ValueError(f'Unsafe or duplicate package entry: {name}')
            files[name] = package.extractfile(member).read()
    required = {'Cargo.toml', 'Cargo.toml.orig', 'Cargo.lock', 'LICENSE', 'src/lib.rs',
                '.cargo_vcs_info.json', 'docs/rustdoc/features.md',
                'docs/rustdoc/pagination.md', 'docs/rustdoc/updates.md'}
    if not required <= files.keys():
        raise ValueError('Package is missing required metadata, source, license or bundled guides')
    vcs = json.loads(files['.cargo_vcs_info.json'])
    if vcs.get('git', {}).get('sha1') != sha or vcs.get('git', {}).get('dirty') or vcs.get('path_in_vcs') != '':
        raise ValueError('Package VCS metadata does not match the clean release commit')
    for name, data in files.items():
        if name.startswith(('.agents/', '.github/', '.git/', 'scripts/', 'docs/media/',
                            'docs/screenshots/', 'docs/benchmarks/', 'docs/integration/')) or name in {
                                'AGENTS.md', 'docs/RELEASING.md', 'docs/index.html', 'docs/.nojekyll'}:
            raise ValueError(f'Excluded development file in package: {name}')
        if name not in {'Cargo.toml', '.cargo_vcs_info.json'}:
            original = 'Cargo.toml' if name == 'Cargo.toml.orig' else name
            if data != source(original):
                raise ValueError(f'Packaged source differs from Git: {name}')
    manifest = tomllib.loads(files['Cargo.toml'].decode())
    original = tomllib.loads(files['Cargo.toml.orig'].decode())
    for field in ('name', 'version', 'license', 'edition', 'rust-version'):
        if manifest['package'][field] != original['package'][field]:
            raise ValueError(f'Normalized package metadata differs: {field}')
    if manifest['package']['name'] != 'ratagrid' or manifest['package']['version'] != version:
        raise ValueError('Package name/version differs from the release')
    if manifest['package']['license'] != 'MIT':
        raise ValueError('Expected the MIT license')
    dependencies = manifest.get('dependencies', {})
    if 'ratatui-core' not in dependencies or 'ratatui' in dependencies:
        raise ValueError('Library must depend on ratatui-core, with full Ratatui only for development')
    lock = tomllib.loads(files['Cargo.lock'].decode())
    roots = [p for p in lock['package'] if p['name'] == 'ratagrid' and 'source' not in p]
    if len(roots) != 1 or roots[0]['version'] != version:
        raise ValueError('Packaged lockfile does not match the release')
    return {'version': version, 'sha': sha, 'sha256': hashlib.sha256(archive.read_bytes()).hexdigest(),
            'files': len(files), 'registry_upload': False}


def package():
    manifest = tomllib.loads((ROOT / 'Cargo.toml').read_text())
    version = manifest['package']['version']
    run(sys.executable, str(ROOT / 'scripts/check-release.py'), 'v' + version)
    sha = git('rev-parse', 'HEAD').decode().strip()
    tag = subprocess.run(['git', 'rev-parse', '--verify', '--quiet', f'refs/tags/v{version}^{{commit}}'],
                         cwd=ROOT, capture_output=True, text=True)
    validate_source(git('status', '--porcelain').decode(), sha, tag.stdout.strip() if tag.returncode == 0 else None)
    # One Cargo packaging/build verification; the dry run never uploads.
    run('cargo', 'publish', '--locked', '--dry-run', '--registry', 'crates-io')
    metadata = json.loads(subprocess.check_output(['cargo', 'metadata', '--no-deps', '--locked', '--format-version=1'], cwd=ROOT))
    target = Path(metadata['target_directory'])
    archive = target / 'package' / f'ratagrid-{version}.crate'
    report = inspect_archive(archive, version, sha, lambda name: git('show', f'{sha}:{name}'))
    output = target / 'release-preflight.json'
    output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))
    print(f'Package verified; evidence saved to {output}. No upload or tag was created.')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command', choices=['check', 'package'], nargs='?', default='check')
    args = parser.parse_args()
    try:
        {'check': check, 'package': package}[args.command]()
    except (ValueError, KeyError, OSError, subprocess.CalledProcessError, tarfile.TarError) as error:
        parser.exit(1, f'{error}\n')


if __name__ == '__main__':
    main()
