#!/usr/bin/env python3
"""Compare fresh and unchanged warm CI Cargo builds in isolated target directories.

Runs sequentially, removes only its own temporary build directories, and keeps
logs plus JSON results. Offline Cargo excludes downloads from measured time.
"""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import statistics
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
PROFILES = {
    'baseline': {'CARGO_PROFILE_DEV_DEBUG': '2', 'CARGO_PROFILE_TEST_DEBUG': '2', 'CARGO_INCREMENTAL': '1'},
    'lean-ci': {'CARGO_PROFILE_DEV_DEBUG': '1', 'CARGO_PROFILE_TEST_DEBUG': '1', 'CARGO_INCREMENTAL': '0'},
}
STAGES = [
    ('clippy', ['cargo', 'clippy', '--all-targets', '--locked', '--offline', '--', '-D', 'warnings']),
    ('tests', ['cargo', 'test', '--all-targets', '--locked', '--offline']),
    ('doctests', ['cargo', 'test', '--doc', '--locked', '--offline']),
    ('rustdoc', ['cargo', 'doc', '--no-deps', '--locked', '--offline']),
]


def output(command):
    return subprocess.check_output(command, cwd=ROOT, text=True).strip()


def artifact_bytes(target):
    return sum(path.stat().st_size for path in target.rglob('*') if path.is_file() and not path.is_symlink())


def save(path, report):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--profiles', nargs='+', choices=PROFILES, default=list(PROFILES))
    parser.add_argument('--samples', type=int, default=2, help='fresh trials per profile')
    parser.add_argument('--warm-runs', type=int, default=2, help='unchanged warm repeats per fresh trial')
    parser.add_argument('--include-release', action='store_true', help='also build all release examples')
    parser.add_argument('--output', type=Path, default=ROOT / 'target/build-benchmark/results.json')
    args = parser.parse_args()
    if args.samples < 1 or args.warm_runs < 0:
        parser.error('samples must be positive and warm-runs must be nonnegative')
    scratch = ROOT / 'target/build-benchmark'
    scratch.mkdir(parents=True, exist_ok=True)
    logs = scratch / 'logs'
    logs.mkdir(exist_ok=True)
    stages = list(STAGES)
    if args.include_release:
        stages.append(('release-examples', ['cargo', 'build', '--release', '--examples', '--locked', '--offline']))
    report = {
        'measured_at_utc': datetime.now(timezone.utc).isoformat(),
        'source_commit': output(['git', 'rev-parse', 'HEAD']),
        'cargo_lock_sha256': hashlib.sha256((ROOT / 'Cargo.lock').read_bytes()).hexdigest(),
        'platform': platform.platform(),
        'machine': platform.machine(),
        'logical_cpus': os.cpu_count(),
        'rustc': output(['rustc', '-Vv']),
        'cargo': output(['cargo', '-V']),
        'profiles': PROFILES,
        'selected_profiles': args.profiles,
        'requested_samples': args.samples,
        'requested_warm_runs': args.warm_runs,
        'stages': {name: command for name, command in stages},
        'method': 'Sequential fresh target directories; alternating profile order; warm repeats keep every artifact. Registry already populated; offline. Release and setup excluded unless requested.',
        'trials': [],
    }
    for sample in range(args.samples):
        # AB, BA balances ordering effects without concurrent compiler workloads.
        profiles = args.profiles if sample % 2 == 0 else list(reversed(args.profiles))
        for profile in profiles:
            required_mib = (800 if profile == 'baseline' else 300) + (250 if args.include_release else 0)
            if shutil.disk_usage(scratch).free < required_mib * 1024 ** 2:
                raise SystemExit(f'Need at least {required_mib} MiB free for an isolated {profile} build')
            with tempfile.TemporaryDirectory(prefix=f'{profile}-{sample + 1}-', dir=scratch) as temporary:
                target = Path(temporary)
                env = dict(os.environ, **PROFILES[profile], CARGO_TARGET_DIR=str(target), CARGO_BUILD_JOBS=str(os.cpu_count() or 1), RUSTDOCFLAGS='-D warnings')
                trial = {'profile': profile, 'sample': sample + 1, 'runs': []}
                report['trials'].append(trial)
                for run in range(args.warm_runs + 1):
                    measured = {'cache': 'cold' if run == 0 else 'warm', 'repeat': run, 'seconds': {}}
                    trial['runs'].append(measured)
                    for name, command in stages:
                        log = logs / f'{profile}-{sample + 1}-{run}-{name}.log'
                        start = time.perf_counter()
                        with log.open('w', encoding='utf-8') as stream:
                            result = subprocess.run(command, cwd=ROOT, env=env, stdout=stream, stderr=subprocess.STDOUT)
                        seconds = time.perf_counter() - start
                        measured['seconds'][name] = round(seconds, 4)
                        save(args.output, report)
                        print(f'{profile} sample {sample + 1} {measured["cache"]} {run}: {name} {seconds:.2f}s', flush=True)
                        if result.returncode:
                            raise SystemExit(f'{name} failed; see {log}')
                    measured['total_seconds'] = round(sum(measured['seconds'].values()), 4)
                    measured['artifact_bytes'] = artifact_bytes(target)
                    save(args.output, report)
    report['summary'] = {}
    for profile in args.profiles:
        report['summary'][profile] = {}
        for cache in ['cold', 'warm']:
            runs = [run for trial in report['trials'] if trial['profile'] == profile for run in trial['runs'] if run['cache'] == cache]
            if runs:
                report['summary'][profile][cache] = {
                    'samples': len(runs),
                    'median_seconds': round(statistics.median(run['total_seconds'] for run in runs), 4),
                    'median_artifact_bytes': statistics.median(run['artifact_bytes'] for run in runs),
                    'median_stage_seconds': {name: round(statistics.median(run['seconds'][name] for run in runs), 4) for name, _ in stages},
                }
    save(args.output, report)
    print(json.dumps(report['summary'], indent=2), flush=True)


if __name__ == '__main__':
    main()
