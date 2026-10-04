#!/usr/bin/env python3
"""Build the dependency-free Pages site from its template and tested integration."""
import argparse
from html import escape
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CODE_PLACEHOLDER = "__CODE__"


def render_outputs(root):
    """Read canonical sources and return the complete generated file contents."""
    source = (root / "examples/positions.rs").read_text(encoding="utf-8")
    template = (root / "scripts/templates/pages.html").read_text(encoding="utf-8")
    if template.count(CODE_PLACEHOLDER) != 1:
        raise ValueError("Pages template must contain exactly one __CODE__ placeholder")
    code = escape(source.split("#[cfg(test)]", 1)[0].rstrip())
    return {
        root / "docs/index.html": template.replace(CODE_PLACEHOLDER, code),
        root / "docs/integration/positions.rs": source,
        root / "docs/.nojekyll": "",
    }


def build(root=ROOT, check=False):
    outputs = render_outputs(root)
    for path, content in outputs.items():
        if check:
            if not path.exists() or path.read_text(encoding="utf-8") != content:
                raise SystemExit(f"Outdated {path.relative_to(root)}; run scripts/build_docs.py")
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(content, encoding="utf-8")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="fail if generated Pages files are stale")
    args = parser.parse_args()
    build(check=args.check)
    print("Pages files are current" if args.check else "Built Pages files")


if __name__ == "__main__":
    main()
