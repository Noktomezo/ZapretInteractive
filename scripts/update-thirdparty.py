#!/usr/bin/env python3
from __future__ import annotations

import fnmatch
import hashlib
import io
import json
import urllib.request
from urllib.parse import urlparse
from pathlib import Path
from zipfile import ZipFile

REPO_ROOT = Path(__file__).resolve().parents[1]
THIRDPARTY_DIR = REPO_ROOT / "thirdparty"
HASHES_PATH = THIRDPARTY_DIR / "hashes.json"

BINARY_FILES = [
    "WinDivert.dll",
    "Monkey64.sys",
    "winws.exe",
    "cygwin1.dll",
]

FAKE_FILES = [
    "4pda.bin",
    "dht_find_node.bin",
    "dht_get_peers.bin",
    "discord-ip-discovery-with-port.bin",
    "discord-ip-discovery-without-port.bin",
    "dtls_clienthello_w3_org.bin",
    "http_iana_org.bin",
    "isakmp_initiator_request.bin",
    "max.bin",
    "quic_initial_facebook_com.bin",
    "quic_initial_facebook_com_quiche.bin",
    "quic_initial_rr1---sn-xguxaxjvh-n8me_googlevideo_com_kyber_1.bin",
    "quic_initial_rr1---sn-xguxaxjvh-n8me_googlevideo_com_kyber_2.bin",
    "quic_initial_rr2---sn-gvnuxaxjvh-o8ge_googlevideo_com.bin",
    "quic_initial_rutracker_org.bin",
    "quic_initial_rutracker_org_kyber_1.bin",
    "quic_initial_rutracker_org_kyber_2.bin",
    "quic_initial_vk_com.bin",
    "quic_initial_www_google_com.bin",
    "quic_short_header.bin",
    "stun.bin",
    "t2.bin",
    "tls_clienthello_gosuslugi_ru.bin",
    "tls_clienthello_iana_org.bin",
    "tls_clienthello_max_ru.bin",
    "tls_clienthello_rutracker_org_kyber.bin",
    "tls_clienthello_sberbank_ru.bin",
    "tls_clienthello_vk_com.bin",
    "tls_clienthello_vk_com_kyber.bin",
    "tls_clienthello_www_google_com.bin",
    "tls_clienthello_www_onetrust_com.bin",
    "wireguard_initiation.bin",
    "wireguard_response.bin",
    "zero_1024.bin",
    "zero_256.bin",
    "zero_512.bin",
]

LIST_FILES = [
    "zapret-hosts-google.txt",
    "zapret-hosts-user-exclude.txt",
    "zapret-ip-user.txt",
]

MODULE_FILES = [
    "dnscrypt-proxy/dnscrypt-proxy.exe",
    "tg-ws-proxy-rs/tg-ws-proxy.exe",
]

UPSTREAMS = [
    (
        "https://raw.githubusercontent.com/StressOzz/Zapret-Manager/refs/heads/main/zapret-hosts-user-exclude.txt",
        THIRDPARTY_DIR / "lists" / "zapret-hosts-user-exclude.txt",
    ),
    (
        "https://raw.githubusercontent.com/Noktomezo/RussiaFancyLists/refs/heads/main/lists/ipsets/full-and-cdn.lst",
        THIRDPARTY_DIR / "lists" / "zapret-ip-user.txt",
    ),
    (
        "https://github.com/bol-van/zapret-win-bundle/raw/refs/heads/master/zapret-winws/winws.exe",
        THIRDPARTY_DIR / "winws.exe",
    ),
    (
        "https://github.com/bol-van/zapret-win-bundle/raw/refs/heads/master/windivert-hide/Monkey64.sys",
        THIRDPARTY_DIR / "Monkey64.sys",
    ),
    (
        "https://github.com/bol-van/zapret-win-bundle/raw/refs/heads/master/windivert-hide/WinDivert.dll",
        THIRDPARTY_DIR / "WinDivert.dll",
    ),
    (
        "https://github.com/bol-van/zapret-win-bundle/raw/refs/heads/master/zapret-winws/cygwin1.dll",
        THIRDPARTY_DIR / "cygwin1.dll",
    ),
]

GITHUB_RELEASE_ZIP_UPSTREAMS = [
    {
        "repo": "DNSCrypt/dnscrypt-proxy",
        "asset_pattern": "dnscrypt-proxy-win64-*.zip",
        "member_pattern": "*/dnscrypt-proxy.exe",
        "destination": THIRDPARTY_DIR / "modules" / "dnscrypt-proxy" / "dnscrypt-proxy.exe",
    },
    {
        "repo": "valnesfjord/tg-ws-proxy-rs",
        "asset_pattern": "tg-ws-proxy-x86_64-pc-windows-gnu.zip",
        "member_pattern": "tg-ws-proxy.exe",
        "destination": THIRDPARTY_DIR / "modules" / "tg-ws-proxy-rs" / "tg-ws-proxy.exe",
    },
]


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    hasher = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            hasher.update(chunk)
    return hasher.hexdigest()


def download(url: str) -> bytes:
    parsed = urlparse(url)
    if parsed.scheme.lower() != "https":
        raise ValueError(f"Unsupported URL scheme for managed download: {url}")

    request = urllib.request.Request(
        url,
        headers={"User-Agent": "ZapretInteractive-CI/1.0"},
    )
    with urllib.request.urlopen(request, timeout=120) as response:
        return response.read()


def download_json(url: str) -> dict:
    return json.loads(download(url).decode("utf-8"))


def fetch_latest_release_asset(repo: str, asset_pattern: str) -> bytes:
    release = download_json(f"https://api.github.com/repos/{repo}/releases/latest")
    assets = release.get("assets", [])
    matches = [
        asset for asset in assets
        if fnmatch.fnmatch(asset.get("name", ""), asset_pattern)
    ]

    if not matches:
        available_assets = ", ".join(
            sorted(asset.get("name", "<unnamed>") for asset in assets)
        )
        raise FileNotFoundError(
            f"Latest release asset for {repo} matching {asset_pattern!r} not found. "
            f"Available assets: {available_assets}"
        )

    if len(matches) > 1:
        ambiguous_assets = ", ".join(
            sorted(
                f"{asset.get('name', '<unnamed>')} ({asset.get('browser_download_url', '<no-url>')})"
                for asset in matches
            )
        )
        raise ValueError(
            f"Latest release asset lookup for {repo} with pattern {asset_pattern!r} "
            f"is ambiguous: {ambiguous_assets}"
        )

    asset = matches[0]
    asset_name = asset.get("name", "")
    asset_url = asset.get("browser_download_url")
    if not asset_url:
        raise ValueError(
            f"Latest release asset {asset_name} for {repo} has no browser_download_url"
        )
    return download(asset_url)


def extract_zip_member(data: bytes, member_pattern: str) -> bytes:
    with ZipFile(io.BytesIO(data)) as archive:
        matches = [
            member_name
            for member_name in archive.namelist()
            if fnmatch.fnmatch(member_name.replace("\\", "/"), member_pattern)
        ]

        if not matches:
            raise FileNotFoundError(f"Zip member matching {member_pattern!r} not found")

        if len(matches) > 1:
            normalized_matches = ", ".join(
                sorted(member_name.replace("\\", "/") for member_name in matches)
            )
            raise ValueError(
                f"Zip member lookup with pattern {member_pattern!r} is ambiguous: "
                f"{normalized_matches}"
            )

        with archive.open(matches[0]) as handle:
            return handle.read()


def normalize_download(destination: Path, data: bytes) -> bytes:
    if destination.name != "zapret-hosts-user-exclude.txt":
        return data

    lines = data.decode("utf-8").splitlines()
    seen: set[str] = set()
    normalized: list[str] = []

    for line in lines:
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            normalized.append(line)
            continue
        if stripped in seen:
            continue
        seen.add(stripped)
        normalized.append(line)

    return ("\n".join(normalized) + "\n").encode("utf-8")


def write_if_changed(path: Path, data: bytes) -> bool:
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists() and sha256_file(path) == sha256_bytes(data):
        return False

    temp_path = path.with_suffix(path.suffix + ".tmp")
    temp_path.write_bytes(data)
    temp_path.replace(path)
    return True


def build_hash_manifest() -> dict[str, str]:
    manifest: dict[str, str] = {}

    for name in BINARY_FILES:
        path = THIRDPARTY_DIR / name
        if not path.is_file():
            raise FileNotFoundError(f"Missing managed binary: {path}")
        manifest[f"binaries:{name}"] = sha256_file(path)

    for name in FAKE_FILES:
        path = THIRDPARTY_DIR / "fake" / name
        if not path.is_file():
            raise FileNotFoundError(f"Missing managed fake file: {path}")
        manifest[f"fake:{name}"] = sha256_file(path)

    for name in LIST_FILES:
        path = THIRDPARTY_DIR / "lists" / name
        if not path.is_file():
            raise FileNotFoundError(f"Missing managed list: {path}")
        manifest[f"lists:{name}"] = sha256_file(path)

    for name in MODULE_FILES:
        path = THIRDPARTY_DIR / "modules" / name
        if not path.is_file():
            raise FileNotFoundError(f"Missing managed module binary: {path}")
        manifest[f"modules:{name}"] = sha256_file(path)

    return manifest


def main() -> None:
    changed_paths: list[str] = []

    for url, destination in UPSTREAMS:
        data = normalize_download(destination, download(url))
        if write_if_changed(destination, data):
            changed_paths.append(destination.relative_to(REPO_ROOT).as_posix())

    for upstream in GITHUB_RELEASE_ZIP_UPSTREAMS:
        archive_bytes = fetch_latest_release_asset(
            upstream["repo"],
            upstream["asset_pattern"],
        )
        extracted_bytes = extract_zip_member(archive_bytes, upstream["member_pattern"])
        if write_if_changed(upstream["destination"], extracted_bytes):
            changed_paths.append(upstream["destination"].relative_to(REPO_ROOT).as_posix())

    manifest = build_hash_manifest()
    hashes_payload = json.dumps(manifest, ensure_ascii=False, indent=2, sort_keys=True) + "\n"
    if write_if_changed(HASHES_PATH, hashes_payload.encode("utf-8")):
        changed_paths.append(HASHES_PATH.relative_to(REPO_ROOT).as_posix())

    if changed_paths:
        print("Updated managed files:")
        for path in changed_paths:
            print(f" - {path}")
    else:
        print("thirdparty is already up to date")


if __name__ == "__main__":
    main()
