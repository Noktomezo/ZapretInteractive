const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

const REPO_ROOT = path.resolve(__dirname, '..');
const THIRDPARTY_DIR = path.join(REPO_ROOT, 'thirdparty');

const DNSCRYPT_URL = "https://github.com/DNSCrypt/dnscrypt-proxy/releases/download/2.1.15/dnscrypt-proxy-linux_x86_64-2.1.15.tar.gz";
const TG_WS_PROXY_URL = "https://github.com/valnesfjord/tg-ws-proxy-rs/releases/download/v1.5.0/tg-ws-proxy-x86_64-unknown-linux-musl.tar.gz";

async function downloadFile(url, dest) {
    console.log(`Downloading ${url}...`);
    const response = await fetch(url);
    if (!response.ok) throw new Error(`HTTP error! status: ${response.status}`);
    const buffer = Buffer.from(await response.arrayBuffer());
    fs.writeFileSync(dest, buffer);
    console.log(`Saved to ${dest}`);
}

async function main() {
    const tempDir = path.join(REPO_ROOT, 'temp_download');
    if (!fs.existsSync(tempDir)) {
        fs.mkdirSync(tempDir, { recursive: true });
    }

    const dnscryptTar = path.join(tempDir, 'dnscrypt.tar.gz');
    const tgProxyTar = path.join(tempDir, 'tgproxy.tar.gz');

    try {
        await downloadFile(DNSCRYPT_URL, dnscryptTar);
        await downloadFile(TG_WS_PROXY_URL, tgProxyTar);

        // Extract DNSCrypt-proxy
        console.log("Extracting DNSCrypt-proxy...");
        const dnscryptExtractDir = path.join(tempDir, 'dnscrypt_extracted');
        fs.mkdirSync(dnscryptExtractDir, { recursive: true });
        execSync(`tar -xzf "${dnscryptTar}" -C "${dnscryptExtractDir}"`);

        // Find the binary - it's located in the folder `linux-x86_64/dnscrypt-proxy`
        const dnscryptDestDir = path.join(THIRDPARTY_DIR, 'modules', 'dnscrypt-proxy');
        fs.mkdirSync(dnscryptDestDir, { recursive: true });

        const dnscryptBinarySrc = path.join(dnscryptExtractDir, 'linux-x86_64', 'dnscrypt-proxy');
        const dnscryptBinaryDst = path.join(dnscryptDestDir, 'dnscrypt-proxy');

        if (fs.existsSync(dnscryptBinarySrc)) {
            fs.copyFileSync(dnscryptBinarySrc, dnscryptBinaryDst);
            console.log(`Successfully extracted dnscrypt-proxy to ${dnscryptBinaryDst}`);
        } else {
            throw new Error(`Could not find dnscrypt-proxy binary in extracted files`);
        }

        // Extract tg-ws-proxy-rs
        console.log("Extracting tg-ws-proxy...");
        const tgProxyExtractDir = path.join(tempDir, 'tgproxy_extracted');
        fs.mkdirSync(tgProxyExtractDir, { recursive: true });
        execSync(`tar -xzf "${tgProxyTar}" -C "${tgProxyExtractDir}"`);

        const tgProxyDestDir = path.join(THIRDPARTY_DIR, 'modules', 'tg-ws-proxy-rs');
        fs.mkdirSync(tgProxyDestDir, { recursive: true });

        const tgProxyBinarySrc = path.join(tgProxyExtractDir, 'tg-ws-proxy');
        const tgProxyBinaryDst = path.join(tgProxyDestDir, 'tg-ws-proxy');

        if (fs.existsSync(tgProxyBinarySrc)) {
            fs.copyFileSync(tgProxyBinarySrc, tgProxyBinaryDst);
            console.log(`Successfully extracted tg-ws-proxy to ${tgProxyBinaryDst}`);
        } else {
            throw new Error(`Could not find tg-ws-proxy binary in extracted files`);
        }

    } finally {
        // Clean up
        console.log("Cleaning up temp files...");
        try {
            fs.rmSync(tempDir, { recursive: true, force: true });
        } catch (e) {
            console.error("Cleanup error:", e.message);
        }
    }
    console.log("Done!");
}

main().catch(err => {
    console.error("Error:", err);
    process.exit(1);
});
