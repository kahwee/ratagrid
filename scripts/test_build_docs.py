#!/usr/bin/env python3
"""Regression tests for Pages generation; only temporary fixtures are modified."""
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

import build_docs


class PagesBuildTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        (self.root / "examples").mkdir()
        (self.root / "scripts/templates").mkdir(parents=True)
        self.source = 'fn main() { println!("東京 <tag> & \"quoted\""); }\n\n#[cfg(test)]\nmod tests {}\n'
        (self.root / "examples/positions.rs").write_text(self.source, encoding="utf-8")
        self.template = self.root / "scripts/templates/pages.html"
        self.template.write_text('<pre><code>__CODE__</code></pre>\n', encoding="utf-8")

    def test_escapes_markup_preserves_unicode_and_omits_only_snippet_tests(self):
        build_docs.build(self.root)
        page = (self.root / "docs/index.html").read_text(encoding="utf-8")
        self.assertIn('東京 &lt;tag&gt; &amp;', page)
        self.assertIn('&quot;', page)
        self.assertNotIn('#[cfg(test)]', page)
        self.assertEqual((self.root / "docs/integration/positions.rs").read_text(encoding="utf-8"), self.source)
        self.assertEqual((self.root / "docs/.nojekyll").read_bytes(), b'')
        build_docs.build(self.root, check=True)

    def test_check_rejects_each_stale_output_without_rewriting_any_file(self):
        for relative in ['docs/index.html', 'docs/integration/positions.rs', 'docs/.nojekyll']:
            with self.subTest(output=relative):
                build_docs.build(self.root)
                stale = self.root / relative
                stale.write_text('stale', encoding="utf-8")
                before = {p: p.read_bytes() for p in build_docs.render_outputs(self.root)}
                with self.assertRaisesRegex(SystemExit, f'Outdated {relative}'):
                    build_docs.build(self.root, check=True)
                self.assertEqual({p: p.read_bytes() for p in before}, before)

    def test_check_does_not_create_missing_outputs(self):
        with self.assertRaisesRegex(SystemExit, 'Outdated docs/index.html'):
            build_docs.build(self.root, check=True)
        self.assertFalse((self.root / "docs").exists())

    def test_template_requires_one_placeholder_before_writing_outputs(self):
        for template in ['<pre></pre>', '__CODE__ __CODE__']:
            with self.subTest(template=template):
                self.template.write_text(template, encoding="utf-8")
                with self.assertRaisesRegex(ValueError, 'exactly one'):
                    build_docs.build(self.root)
                self.assertFalse((self.root / "docs").exists())

    def test_regeneration_uses_changed_source_and_template(self):
        build_docs.build(self.root)
        self.template.write_text('<article>__CODE__</article>', encoding="utf-8")
        (self.root / "examples/positions.rs").write_text('fn changed() {}\n', encoding="utf-8")
        with self.assertRaises(SystemExit):
            build_docs.build(self.root, check=True)
        build_docs.build(self.root)
        self.assertEqual((self.root / "docs/index.html").read_text(encoding="utf-8"), '<article>fn changed() {}</article>')
        build_docs.build(self.root, check=True)

    def test_cli_check_works_outside_repository(self):
        result = subprocess.run(
            [sys.executable, str(build_docs.ROOT / "scripts/build_docs.py"), '--check'],
            cwd=self.root, capture_output=True, text=True,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('Pages files are current', result.stdout)


if __name__ == "__main__":
    unittest.main()
