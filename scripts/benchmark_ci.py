#!/usr/bin/env python3
"""Collect hosted-runner benchmarks and publish source-linked public reports."""
import argparse
import csv
from datetime import datetime, timezone
import io
import json
import math
import os
from pathlib import Path
import platform
import shutil
import statistics
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
PUBLIC = ROOT / 'docs/benchmarks/github'
START = '<!-- github-benchmarks:start -->'
END = '<!-- github-benchmarks:end -->'
PLATFORMS = ('linux', 'macos', 'windows')
KEYS = ('case', 'total_rows', 'resident_rows', 'columns', 'width', 'height', 'operation')
METHOD = 'stress-v1-three-process-medians;lean-ci-two-fresh-one-warm'


def output(command):
    return subprocess.check_output(command, cwd=ROOT, text=True).strip()


def save(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2) + '\n', encoding='utf-8')


def aggregate(texts):
    """Require identical cases across trials; never publish partial samples."""
    trials = []
    for text in texts:
        trial = {}
        for row in csv.DictReader(io.StringIO(text)):
            key = tuple(row[name] for name in KEYS)
            values = {name: float(row[name]) for name in ('p50_ms', 'p95_ms', 'max_ms')}
            if key in trial or any(not math.isfinite(v) or v < 0 for v in values.values()):
                raise ValueError('Duplicate case or invalid timing')
            trial[key] = dict(row, **values)
        if not trial:
            raise ValueError('Empty stress output')
        trials.append(trial)
    if len(trials) != 3 or any(t.keys() != trials[0].keys() for t in trials):
        raise ValueError('Expected three complete trials with identical cases')
    return [dict(row, **{name: statistics.median(t[key][name] for t in trials)
                        for name in ('p50_ms', 'p95_ms', 'max_ms')})
            for key, row in trials[0].items()]


def regressions(report, baseline):
    if not baseline:
        return 'No frozen GitHub baseline yet.', []
    fields = ('method', 'runner', 'machine', 'logical_cpus', 'rustc')
    if any(report[k] != baseline[k] for k in fields):
        return 'Baseline environment/method differs; comparison skipped.', []
    previous = {tuple(row[k] for k in KEYS): row for row in baseline['runtime']}
    if set(previous) != {tuple(row[k] for k in KEYS) for row in report['runtime']}:
        return 'Benchmark cases differ; comparison skipped.', []
    alerts = []
    for row in report['runtime']:
        old = previous[tuple(row[k] for k in KEYS)]['p50_ms']
        new = row['p50_ms']
        # Both criteria must hold; ignore timer noise in near-zero operations.
        if new > old * 1.25 and new - old > 0.05:
            alerts.append({'case': {k: row[k] for k in KEYS},
                           'baseline_ms': old, 'current_ms': new,
                           'increase_percent': round((new / old - 1) * 100, 1) if old else None})
    return f'Compared with frozen baseline {baseline["source_commit"][:7]}.', alerts


def markdown(report):
    rows = ['| Workload | Median |', '| --- | ---: |']
    for name, case, total, operation in (
        ('1M rows: draw', 'owned', '1000000', 'render_selected'),
        ('1M rows: numeric sort', 'owned', '1000000', 'sort_latency_ascending'),
        ('1M rows: 50 large moves + draw', 'owned', '1000000', 'sorted_50_cross_dataset_and_render'),
        ('100M virtual / 50 resident: draw', 'external_virtual', '100000000', 'render_selected'),
    ):
        row = next(r for r in report['runtime'] if r['case'] == case and r['total_rows'] == total
                   and r['operation'] == operation and r['width'] == '132')
        rows.append(f'| {name} | {row["p50_ms"]:.3f} ms |')
    summary = report['builds']['summary']['lean-ci']
    rows.extend([
        f'| Fresh Cargo stages | {summary["cold"]["median_seconds"]:.2f} s |',
        f'| Unchanged warm Cargo stages | {summary["warm"]["median_seconds"]:.2f} s |',
    ])
    return '\n'.join(rows)


def collect(destination):
    destination.mkdir(parents=True, exist_ok=True)
    report = {
        'method': METHOD,
        'measured_at_utc': datetime.now(timezone.utc).isoformat(),
        'source_commit': output(['git', 'rev-parse', 'HEAD']),
        'platform': os.environ['BENCHMARK_PLATFORM'],
        'runner': os.environ['BENCHMARK_RUNNER'],
        'runner_image': os.environ.get('ImageVersion', 'unknown'),
        'machine': platform.machine(),
        'logical_cpus': os.cpu_count(),
        'os': platform.platform(),
        'rustc': output(['rustc', '-Vv']),
        'run_id': os.environ['GITHUB_RUN_ID'],
        'run_attempt': os.environ['GITHUB_RUN_ATTEMPT'],
        'run_url': f'https://github.com/{os.environ["GITHUB_REPOSITORY"]}/actions/runs/{os.environ["GITHUB_RUN_ID"]}/attempts/{os.environ["GITHUB_RUN_ATTEMPT"]}',
    }
    subprocess.run(['cargo', 'build', '--release', '--locked', '--offline', '--example', 'playground'], cwd=ROOT, check=True)
    binary = ROOT / 'target/release/examples' / ('playground.exe' if os.name == 'nt' else 'playground')
    texts = []
    for trial in range(1, 4):
        with (destination / f'stress-{trial}.csv').open('w', encoding='utf-8', newline='') as stream, \
                (destination / f'stress-{trial}.log').open('w', encoding='utf-8') as log:
            subprocess.run([str(binary), '--stress'], cwd=ROOT, stdout=stream, stderr=log, check=True)
        texts.append((destination / f'stress-{trial}.csv').read_text(encoding='utf-8'))
    report['runtime'] = aggregate(texts)
    subprocess.run([sys.executable, str(ROOT / 'scripts/benchmark_builds.py'),
                    '--profiles', 'lean-ci', '--samples', '2', '--warm-runs', '1',
                    '--output', str(destination / 'builds.json')], cwd=ROOT, check=True)
    report['builds'] = json.loads((destination / 'builds.json').read_text(encoding='utf-8'))
    shutil.copytree(ROOT / 'target/build-benchmark/logs', destination / 'build-logs', dirs_exist_ok=True)
    baseline_path = PUBLIC / report['platform'] / 'baseline.json'
    baseline = json.loads(baseline_path.read_text(encoding='utf-8')) if baseline_path.exists() else None
    report['comparison'], report['regressions'] = regressions(report, baseline)
    save(destination / 'results.json', report)
    text = f'## {report["runner"]} benchmarks\n\nSource: `{report["source_commit"]}`\n\n{markdown(report)}\n\n{report["comparison"]}\n'
    for alert in report['regressions']:
        message = f'{alert["case"]}: {alert["baseline_ms"]:.3f} -> {alert["current_ms"]:.3f} ms'
        text += f'\n- Possible regression: {message}\n'
        print(f'::warning title=Possible runtime regression::{message}', flush=True)
    text += '\nAdvisory: hosted hardware/load varies; rerun before treating a slowdown as a code regression.\n'
    (destination / 'summary.md').write_text(text, encoding='utf-8')
    with Path(os.environ['GITHUB_STEP_SUMMARY']).open('a', encoding='utf-8') as stream:
        stream.write(text)


def publish(source):
    reports = [json.loads(p.read_text(encoding='utf-8')) for p in sorted(source.glob('benchmark-*/results.json'))]
    if len(reports) != 3 or {r['platform'] for r in reports} != set(PLATFORMS):
        raise ValueError('Publish requires all three successful platform reports')
    identity = {(r['source_commit'], r['run_id'], r['run_attempt'], r['method']) for r in reports}
    if len(identity) != 1:
        raise ValueError('Reports must describe the same source and workflow attempt')
    readme = ROOT / 'README.md'
    original = readme.read_text(encoding='utf-8')
    if original.count(START) != 1 or original.count(END) != 1 or original.index(START) >= original.index(END):
        raise ValueError('README benchmark markers missing or ambiguous')
    reports.sort(key=lambda r: PLATFORMS.index(r['platform']))
    first = reports[0]
    run_key = f'{first["run_id"]}-{first["run_attempt"]}'
    # Keep compact, permanent raw CSV/JSON history; bulky build logs stay in artifacts.
    for report in reports:
        directory = PUBLIC / 'runs' / run_key / report['platform']
        if directory.exists():
            raise ValueError(f'Historical run already exists: {directory}')
        directory.mkdir(parents=True)
        for filename in ('results.json', 'builds.json', 'stress-1.csv', 'stress-2.csv', 'stress-3.csv'):
            shutil.copyfile(source / f'benchmark-{report["platform"]}' / filename, directory / filename)
        save(PUBLIC / report['platform'] / 'latest.json', report)
        baseline = PUBLIC / report['platform'] / 'baseline.json'
        if not baseline.exists():
            save(baseline, report)
    lines = [START, '', f'Measured {first["measured_at_utc"][:10]} on GitHub-hosted runners at '
             f'[`{first["source_commit"][:7]}`](https://github.com/kahwee/ratagrid/commit/{first["source_commit"]}). '
             f'[Workflow and logs]({first["run_url"]}) · [Raw CSV/JSON](docs/benchmarks/github/runs/{run_key}).', '',
             '| Runner | 1M draw | 1M numeric sort | 1M: 50 large moves + draw | 100M virtual / 50 resident draw | Fresh / warm Cargo stages |',
             '| --- | ---: | ---: | ---: | ---: | ---: |']
    for report in reports:
        values = [line.split('|')[2].strip() for line in markdown(report).splitlines()[2:]]
        lines.append(f'| {report["runner"]} ({report["machine"]}) | ' + ' | '.join(values[:4]) + f' | {values[4]} / {values[5]} |')
    lines.extend(['', 'Release-mode, in-memory runtime medians from three process runs; terminal I/O and real database/network work excluded. '
                  'Build timings cover Clippy, tests, doctests and rustdoc with pre-fetched dependencies; setup and release compilation excluded. '
                  'Fresh/warm build figures are medians of two runs each. Compare within a platform; hosted hardware/load varies.', ''])
    for report in reports:
        count = len(report['regressions'])
        lines.append(f'- {report["runner"]}: {report["comparison"]} {count} possible runtime regression(s).')
    lines.extend(['', 'Slowdown warnings require both >25% and >0.05 ms against the frozen baseline; they are advisory. '
                  '[Method, triggers and baseline policy](docs/BENCHMARKS.md).', '', END])
    readme.write_text(original[:original.index(START)] + '\n'.join(lines) + original[original.index(END) + len(END):], encoding='utf-8')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest='command', required=True)
    commands.add_parser('collect').add_argument('--output', type=Path, required=True)
    commands.add_parser('publish').add_argument('--input', type=Path, required=True)
    args = parser.parse_args()
    if args.command == 'collect':
        collect(args.output.resolve())
    else:
        publish(args.input.resolve())


if __name__ == '__main__':
    main()
