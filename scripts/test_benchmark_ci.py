#!/usr/bin/env python3
"""Synthetic checks of benchmark completeness, alerts and publication."""
import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import benchmark_ci as bench

HEADER = ','.join(bench.KEYS) + ',samples,p50_ms,p95_ms,max_ms,record_and_order_bytes\n'


def csv_text(value):
    cases = [('owned', '1000000', 'render_selected'),
             ('owned', '1000000', 'sort_latency_ascending'),
             ('owned', '1000000', 'sorted_50_cross_dataset_and_render'),
             ('external_virtual', '100000000', 'render_selected')]
    return HEADER + ''.join(f'{case},{total},50,9,132,34,{operation},100,{value},{value},{value},2800\n'
                            for case, total, operation in cases)


def report(platform='linux'):
    return dict(method=bench.METHOD, runner=platform, platform=platform, machine='x64',
                logical_cpus=4, rustc='test-rustc', source_commit='a' * 40, run_id='123',
                run_attempt='1', measured_at_utc='2026-10-04',
                run_url='https://github.com/kahwee/ratagrid/actions/runs/123/attempts/1',
                runtime=bench.aggregate([csv_text(1)] * 3), comparison='No baseline yet.',
                regressions=[], builds={'summary': {'lean-ci': {
                    'cold': {'median_seconds': 10}, 'warm': {'median_seconds': 2}}}})


class ReportingTests(unittest.TestCase):
    def test_median_and_incomplete_trials(self):
        self.assertEqual(bench.aggregate([csv_text(n) for n in [1, 100, 3]])[0]['p50_ms'], 3)
        for inputs in ([csv_text(1)] * 2, [csv_text(1)] * 2 + [HEADER],
                       [csv_text(1)] * 2 + [csv_text(float('nan'))],
                       [csv_text(1)] * 2 + [csv_text(1).rsplit('\n', 2)[0] + '\n']):
            with self.assertRaises(ValueError):
                bench.aggregate(inputs)

    def test_alert_requires_relative_and_absolute_slowdown(self):
        base = report()
        current = copy.deepcopy(base)
        current['runtime'][0]['p50_ms'] = 1.3
        self.assertEqual(len(bench.regressions(current, base)[1]), 1)
        current['runtime'][0]['p50_ms'] = 1.2
        self.assertEqual(bench.regressions(current, base)[1], [])
        base['runtime'][0]['p50_ms'] = 0.001
        current['runtime'][0]['p50_ms'] = 0.01
        self.assertEqual(bench.regressions(current, base)[1], [])
        current['rustc'] = 'different'
        self.assertIn('skipped', bench.regressions(current, base)[0])
        self.assertEqual(bench.regressions(current, None)[1], [])

    def test_publication_requires_complete_matching_reports_and_freezes_baseline(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            readme = root / 'README.md'
            readme.write_text(f'Intro\n{bench.START}\nold\n{bench.END}\nOutro\n')
            source = root / 'input'
            for platform in bench.PLATFORMS:
                folder = source / f'benchmark-{platform}'
                folder.mkdir(parents=True)
                bench.save(folder / 'results.json', report(platform))
                bench.save(folder / 'builds.json', {})
                for n in range(1, 4):
                    (folder / f'stress-{n}.csv').write_text(csv_text(1))
            with patch.object(bench, 'ROOT', root), patch.object(bench, 'PUBLIC', root / 'public'):
                bench.publish(source)
                self.assertIn('1.000 ms', readme.read_text())
                self.assertTrue(readme.read_text().endswith('Outro\n'))
                baseline = root / 'public/linux/baseline.json'
                first = baseline.read_text()
                new = report('linux')
                new['runtime'][0]['p50_ms'] = 2
                new['same_runner_before'] = {'source_commit': 'c' * 40, 'runtime': report()['runtime']}
                for n in range(1, 4):
                    (source / f'benchmark-linux/stress-before-{n}.csv').write_text(csv_text(1))
                for platform in bench.PLATFORMS:
                    updated = new if platform == 'linux' else report(platform)
                    updated['run_id'] = '124'
                    bench.save(source / f'benchmark-{platform}/results.json', updated)
                bench.publish(source)
                self.assertEqual(baseline.read_text(), first)
                self.assertIn('2.000 ms', readme.read_text())
                self.assertIn('1.000 → 2.000 ms (+100.0%)', readme.read_text())
                self.assertTrue((root / 'public/runs/124-1/linux/stress-before-1.csv').exists())
                new['source_commit'] = 'b' * 40
                bench.save(source / 'benchmark-linux/results.json', new)
                with self.assertRaises(ValueError):
                    bench.publish(source)
                (source / 'benchmark-linux/results.json').unlink()
                with self.assertRaises(ValueError):
                    bench.publish(source)


if __name__ == '__main__':
    unittest.main()
