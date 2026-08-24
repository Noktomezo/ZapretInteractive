from __future__ import annotations

import re
import tomllib
from collections import defaultdict
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
STRATEGIES = ROOT / "thirdparty" / "strategies"

CHECKER_FAMILIES = {
    "HTTP": {
        "mode": {"fake", "fakeddisorder", "ipfrag2"},
        "fooling": {"ts", "badsum", "badseq", "md5sig", "datanoack"},
    },
    "TLS": {
        "mode": {"fake", "fakeddisorder", "multidisorder", "ipfrag2"},
        "fooling": {"ts", "badsum", "badseq", "md5sig", "datanoack"},
    },
    "QUIC": {
        "mode": {"fake", "ipfrag2", "udplen"},
        "fooling": {"badsum"},
    },
    "Discord": {
        "mode": {"fake", "ipfrag2", "udplen"},
        "fooling": {"badsum"},
    },
    "DHT": {
        "mode": {"fake", "ipfrag2", "udplen", "tamper"},
        "fooling": {"badsum"},
    },
    "WireGuard": {
        "mode": {"fake", "ipfrag2", "udplen"},
        "fooling": {"badsum"},
    },
}

CATEGORY_GROUPS = {
    "HTTP": {"HTTP"},
    "TLS": {"TCP", "Game TCP", "YouTube"},
    "QUIC": {"QUIC"},
    "Discord": {"Discord + Stun", "Discord Media"},
    "DHT": {"Game UDP"},
    "WireGuard": {"Game UDP"},
}


def values(content: str, option: str) -> set[str]:
    pattern = re.compile(rf"(?m)^--{re.escape(option)}=([^\r\n]+)$")
    found: set[str] = set()
    for raw in pattern.findall(content):
        found.update(part.strip() for part in raw.split(",") if part.strip())
    return found


def load() -> dict[str, list[str]]:
    categories: dict[str, list[str]] = defaultdict(list)
    for path in sorted(STRATEGIES.rglob("*.toml")):
        if path.name == "probe.toml":
            continue
        with path.open("rb") as stream:
            document = tomllib.load(stream)
        categories[str(document.get("category", path.parent.name))].append(
            str(document.get("content", ""))
        )
    return categories


def main() -> None:
    categories = load()
    missing_total = 0
    for family, expected in CHECKER_FAMILIES.items():
        contents = "\n".join(
            content
            for category in CATEGORY_GROUPS[family]
            for content in categories.get(category, [])
        )
        observed = {
            "mode": values(contents, "dpi-desync"),
            "fooling": values(contents, "dpi-desync-fooling"),
        }
        missing = {
            dimension: sorted(required - observed[dimension])
            for dimension, required in expected.items()
            if required - observed[dimension]
        }
        missing_total += sum(len(items) for items in missing.values())
        status = "OK" if not missing else "MISSING"
        print(f"{family:10} {status:7} strategies={sum(len(categories.get(c, [])) for c in CATEGORY_GROUPS[family])}")
        for dimension, items in missing.items():
            print(f"  {dimension}: {', '.join(items)}")
    print(f"\nChecker family gaps: {missing_total}")


if __name__ == "__main__":
    main()
