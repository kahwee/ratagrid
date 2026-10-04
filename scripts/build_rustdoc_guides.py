#!/usr/bin/env python3
"""Bundle repository guides without links to files outside the crate docs."""
import argparse
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]
GUIDES = ("FEATURES", "PAGINATION", "UPDATES")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    for name in GUIDES:
        text = (ROOT / "docs" / f"{name}.md").read_text()
        text = re.sub(r"^!\[.*?\]\(screenshots/[^)]+\)\n", "", text, flags=re.MULTILINE)
        for target in GUIDES:
            text = text.replace(f"]({target}.md)", f"](crate::guides::{target.lower()})")
        output = ROOT / "docs" / "rustdoc" / f"{name.lower()}.md"
        if args.check:
            if not output.exists() or output.read_text() != text:
                raise SystemExit(f"Outdated {output.relative_to(ROOT)}; run scripts/build_rustdoc_guides.py")
        else:
            output.parent.mkdir(exist_ok=True)
            output.write_text(text)


if __name__ == "__main__":
    main()
