import tarfile
import urllib.request
import io
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
THIRDPARTY_DIR = REPO_ROOT / "thirdparty"

DNSCRYPT_URL = "https://github.com/DNSCrypt/dnscrypt-proxy/releases/download/2.1.15/dnscrypt-proxy-linux_x86_64-2.1.15.tar.gz"
TG_WS_PROXY_URL = "https://github.com/valnesfjord/tg-ws-proxy-rs/releases/download/v1.5.0/tg-ws-proxy-x86_64-unknown-linux-musl.tar.gz"

def download_and_extract_tar_member(url: str, member_name_in_tar: str, dest_path: Path):
    print(f"Downloading {url}...")
    req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"})
    with urllib.request.urlopen(req, timeout=120) as response:
        tar_bytes = response.read()

    print(f"Extracting {member_name_in_tar}...")
    dest_path.parent.mkdir(parents=True, exist_ok=True)
    with tarfile.open(fileobj=io.BytesIO(tar_bytes), mode="r:gz") as tar:
        # Find the member by name suffix or exact match
        found = None
        for member in tar.getmembers():
            if member.name.endswith(member_name_in_tar):
                found = member
                break

        if not found:
            raise FileNotFoundError(f"Member {member_name_in_tar} not found in tarball. Available: {[m.name for m in tar.getmembers()]}")

        with tar.extractfile(found) as source_file:
            dest_path.write_bytes(source_file.read())
    print(f"Saved to {dest_path}")

def main():
    # DNSCrypt-proxy Linux tarball contains a "linux-x86_64/dnscrypt-proxy" binary
    download_and_extract_tar_member(
        DNSCRYPT_URL,
        "dnscrypt-proxy",
        THIRDPARTY_DIR / "modules" / "dnscrypt-proxy" / "dnscrypt-proxy"
    )

    # tg-ws-proxy-rs Linux tarball contains a "tg-ws-proxy" binary directly
    download_and_extract_tar_member(
        TG_WS_PROXY_URL,
        "tg-ws-proxy",
        THIRDPARTY_DIR / "modules" / "tg-ws-proxy-rs" / "tg-ws-proxy"
    )
    print("Done!")

if __name__ == "__main__":
    main()
