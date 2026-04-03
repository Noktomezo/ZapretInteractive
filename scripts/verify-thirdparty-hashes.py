#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
THIRDPARTY_DIR = REPO_ROOT / "thirdparty"
HASHES_PATH = THIRDPARTY_DIR / "hashes.json"


def sha256_file(path: Path) -> str:
    hasher = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            hasher.update(chunk)
    return hasher.hexdigest()


def resolve_manifest_path(key: str) -> Path:
    try:
        group, name = key.split(":", 1)
    except ValueError as error:
        raise ValueError(f"Invalid manifest key: {key}") from error

    if group == "binaries":
        return THIRDPARTY_DIR / name
    if group == "fake":
        return THIRDPARTY_DIR / "fake" / name
    if group == "lists":
        return THIRDPARTY_DIR / "lists" / name
    raise ValueError(f"Unsupported manifest group: {group}")


def main() -> int:
    try:
        manifest = json.loads(HASHES_PATH.read_text(encoding="utf-8"))
    except FileNotFoundError:
        print(
            "thirdparty/hashes.json is missing: "
            f"{HASHES_PATH}. Run scripts/update-thirdparty.py before verification."
        )
        return 1

    failures: list[str] = []

    for key, expected_hash in manifest.items():
        path = resolve_manifest_path(key)
        if not path.is_file():
            failures.append(f"MISSING  {key} -> {path.relative_to(REPO_ROOT).as_posix()}")
            continue

        actual_hash = sha256_file(path)
        if actual_hash != expected_hash:
            failures.append(
                f"MISMATCH {key}\n"
                f"  path:     {path.relative_to(REPO_ROOT).as_posix()}\n"
                f"  expected: {expected_hash}\n"
                f"  actual:   {actual_hash}"
            )

    if failures:
        print("thirdparty/hashes.json verification failed")
        for line in failures:
            print(f"- {line}")
        return 1

    print("thirdparty/hashes.json is in sync")
    return 0


if __name__ == "__main__":
    sys.exit(main())
