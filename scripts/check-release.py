"""Check immutable release metadata without compiling or publishing."""
from pathlib import Path
import sys
import tomllib


def validate(manifest, lock, tag):
    package = manifest["package"]
    version = package["version"]
    if package.get("publish") is False:
        raise ValueError("Package disables registry publication")
    if tag != "v" + version:
        raise ValueError("Tag must match Cargo.toml version")
    roots = [p for p in lock["package"] if p["name"] == package["name"] and "source" not in p]
    if len(roots) != 1 or roots[0]["version"] != version:
        raise ValueError("Root Cargo.lock version must match Cargo.toml")
    return package["name"], version


if __name__ == "__main__":
    try:
        manifest = tomllib.loads(Path("Cargo.toml").read_text())
        lock = tomllib.loads(Path("Cargo.lock").read_text())
        name, version = validate(manifest, lock, sys.argv[1])
        print(f"Validated {name}@{version}")
    except (ValueError, KeyError, IndexError) as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
