#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import importlib.util
import json
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
THIRDPARTY_DIR = REPO_ROOT / "thirdparty"
HASHES_PATH = THIRDPARTY_DIR / "hashes.json"
UPDATE_SCRIPT_PATH = Path(__file__).with_name("update-thirdparty.py")


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
    if group == "modules":
        return THIRDPARTY_DIR / "modules" / name
    raise ValueError(f"Unsupported manifest group: {group}")


def load_update_module():
    spec = importlib.util.spec_from_file_location("update_thirdparty", UPDATE_SCRIPT_PATH)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"Failed to load update script: {UPDATE_SCRIPT_PATH}")

    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def build_expected_keys() -> set[str]:
    update_module = load_update_module()
    expected_keys: set[str] = set()

    missing_constants: list[str] = []
    binary_files = getattr(update_module, "BINARY_FILES", None)
    fake_files = getattr(update_module, "FAKE_FILES", None)
    list_files = getattr(update_module, "LIST_FILES", None)
    module_files = getattr(update_module, "MODULE_FILES", None)

    if binary_files is None:
        missing_constants.append("BINARY_FILES")
        binary_files = []
    if fake_files is None:
        missing_constants.append("FAKE_FILES")
        fake_files = []
    if list_files is None:
        missing_constants.append("LIST_FILES")
        list_files = []
    if module_files is None:
        missing_constants.append("MODULE_FILES")
        module_files = []

    if missing_constants:
        raise RuntimeError(
            "update-thirdparty.py is missing required managed file lists: "
            + ", ".join(missing_constants)
        )

    for name in binary_files:
        expected_keys.add(f"binaries:{name}")

    for name in fake_files:
        expected_keys.add(f"fake:{name}")

    for name in list_files:
        expected_keys.add(f"lists:{name}")

    for name in module_files:
        expected_keys.add(f"modules:{name}")

    return expected_keys


def main() -> int:
    try:
        manifest = json.loads(HASHES_PATH.read_text(encoding="utf-8"))
    except FileNotFoundError:
        print(
            "thirdparty/hashes.json is missing: "
            f"{HASHES_PATH}. Run scripts/update-thirdparty.py before verification."
        )
        return 1

    expected_keys = build_expected_keys()
    manifest_keys = set(manifest.keys())
    failures: list[str] = []

    missing_keys = sorted(expected_keys - manifest_keys)
    extra_keys = sorted(manifest_keys - expected_keys)

    if missing_keys:
        failures.append("MISSING KEYS\n  " + "\n  ".join(missing_keys))

    if extra_keys:
        failures.append("EXTRA KEYS\n  " + "\n  ".join(extra_keys))

    for key in sorted(expected_keys & manifest_keys):
        expected_hash = manifest[key]
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
