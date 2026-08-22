use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::Path;

use anyhow::{Context, Result, bail};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const THIRD_PARTY_BASE_URL: &str =
    "https://raw.githubusercontent.com/Noktomezo/ZapretInteractive/main/thirdparty";
const REMOTE_HASHES_URL: &str =
    "https://raw.githubusercontent.com/Noktomezo/ZapretInteractive/main/thirdparty/hashes.json";

const BINARIES: &[&str] = &["WinDivert.dll", "Monkey64.sys", "winws.exe", "cygwin1.dll"];
const MODULE_FILES: &[&str] = &[
    "modules/dnscrypt-proxy/dnscrypt-proxy.exe",
    "modules/dnscrypt-proxy/LICENSE",
    "modules/tg-ws-proxy-rs/tg-ws-proxy.exe",
];

const FAKE_FILES: &[&str] = &[
    "4pda.bin",
    "ACTIVE_DISCORD_UDP.bin",
    "ACTIVE_GAME_UDP.bin",
    "dht_find_node.bin",
    "dht_get_peers.bin",
    "discord-ip-discovery-with-port.bin",
    "discord-ip-discovery-without-port.bin",
    "dtls_clienthello_w3_org.bin",
    "http_iana_org.bin",
    "isakmp_initiator_request.bin",
    "max.bin",
    "quic_initial_4pda.to.bin",
    "quic_initial_5ka_ru.bin",
    "quic_initial_dbankcloud_ru.bin",
    "quic_initial_facebook_com.bin",
    "quic_initial_facebook_com_quiche.bin",
    "quic_initial_rr1---sn-xguxaxjvh-n8me_googlevideo_com_kyber_1.bin",
    "quic_initial_rr1---sn-xguxaxjvh-n8me_googlevideo_com_kyber_2.bin",
    "quic_initial_rr2---sn-gvnuxaxjvh-o8ge_googlevideo_com.bin",
    "quic_initial_rutracker_org.bin",
    "quic_initial_rutracker_org_kyber_1.bin",
    "quic_initial_rutracker_org_kyber_2.bin",
    "quic_initial_rutube_ru.bin",
    "quic_initial_steamcommunity_com.bin",
    "quic_initial_tencent_com.bin",
    "quic_initial_vk_com.bin",
    "quic_initial_www_google_com.bin",
    "quic_short_header.bin",
    "stun.bin",
    "stun2.bin",
    "t2.bin",
    "tls_clienthello_4pda_to.bin",
    "tls_clienthello_5ka_ru.bin",
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
];

const LISTS: &[&str] = &[
    "lists/zapret-hosts-google.txt",
    "lists/zapret-hosts-user-exclude.txt",
    "lists/zapret-ip-user.txt",
];

const DEFAULT_FILTERS: &[&str] = &[
    "filters/windivert_part.dht.txt",
    "filters/windivert_part.discord_media.txt",
    "filters/windivert_part.quic_initial_ietf.txt",
    "filters/windivert_part.stun.txt",
    "filters/windivert_part.wireguard.txt",
];

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AppHealthSnapshot {
    pub binaries_ok: bool,
    pub missing_critical_files: Vec<String>,
    pub available_updates: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DownloadProgress {
    pub current: usize,
    pub total: usize,
    pub filename: String,
}

pub fn compute_sha256(path: &Path) -> Result<String> {
    let file = File::open(path).with_context(|| format!("cannot open {}", path.display()))?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn compute_sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn hash_differs(path: &Path, expected: &str) -> bool {
    compute_sha256(path).map_or(true, |actual| !actual.eq_ignore_ascii_case(expected))
}

fn tracked_paths() -> impl Iterator<Item = String> {
    BINARIES
        .iter()
        .map(|path| (*path).to_owned())
        .chain(MODULE_FILES.iter().map(|path| (*path).to_owned()))
        .chain(FAKE_FILES.iter().map(|path| format!("fake/{path}")))
        .chain(LISTS.iter().map(|path| (*path).to_owned()))
}

fn normalize_remote_hashes(hashes: HashMap<String, String>) -> HashMap<String, String> {
    hashes
        .into_iter()
        .map(|(key, hash)| {
            let path = ["binaries", "fake", "lists", "modules"]
                .into_iter()
                .find_map(|group| {
                    key.strip_prefix(&format!("{group}:")).map(|name| {
                        if group == "binaries" {
                            name.to_owned()
                        } else {
                            format!("{group}/{name}")
                        }
                    })
                })
                .unwrap_or(key);
            (path.replace('\\', "/"), hash)
        })
        .collect()
}

fn load_stored_hashes(resources_dir: &Path) -> Result<HashMap<String, String>> {
    let path = resources_dir.join("hashes.json");
    if !path.is_file() {
        return Ok(HashMap::new());
    }
    let content =
        fs::read_to_string(&path).with_context(|| format!("cannot read {}", path.display()))?;
    serde_json::from_str(&content)
        .map(normalize_remote_hashes)
        .with_context(|| format!("invalid JSON in {}", path.display()))
}

pub fn check_local_health(resources_dir: &Path) -> AppHealthSnapshot {
    let mut missing_critical = Vec::new();

    let stored_hashes = load_stored_hashes(resources_dir).unwrap_or_default();
    for rel in tracked_paths() {
        let full = resources_dir.join(&rel);
        if !full.is_file() {
            missing_critical.push(rel);
        } else if let Some(expected) = stored_hashes.get(&rel)
            && hash_differs(&full, expected)
        {
            missing_critical.push(rel);
        }
    }

    let binaries_ok = missing_critical.is_empty();
    AppHealthSnapshot {
        binaries_ok,
        missing_critical_files: missing_critical,
        available_updates: Vec::new(),
    }
}

pub async fn fetch_remote_hashes(client: &Client) -> Result<HashMap<String, String>> {
    let response = client
        .get(REMOTE_HASHES_URL)
        .send()
        .await
        .context("failed to fetch remote hashes.json")?;
    if !response.status().is_success() {
        bail!("remote hashes.json returned HTTP {}", response.status());
    }
    let map: HashMap<String, String> = response.json().await?;
    Ok(normalize_remote_hashes(map))
}

async fn download_verified_file(
    client: &Client,
    resources_dir: &Path,
    rel_path: &str,
    expected_hash: &str,
) -> Result<()> {
    let url = format!("{THIRD_PARTY_BASE_URL}/{rel_path}");
    let response = client
        .get(&url)
        .send()
        .await
        .with_context(|| format!("failed to download {url}"))?;
    if !response.status().is_success() {
        bail!("downloading {url} returned HTTP {}", response.status());
    }
    let bytes = response.bytes().await?;
    let actual_hash = compute_sha256_bytes(&bytes);
    if !actual_hash.eq_ignore_ascii_case(expected_hash) {
        bail!("hash mismatch for {rel_path}: expected {expected_hash}, got {actual_hash}");
    }

    let destination = resources_dir.join(rel_path);
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = destination.with_extension("tmp");
    fs::write(&temporary, bytes)
        .with_context(|| format!("failed to write {}", temporary.display()))?;
    replace_file(&temporary, &destination)
}

pub async fn repair_default_filters_for_bootstrap(
    client: &Client,
    resources_dir: &Path,
) -> Result<Vec<String>> {
    let mut stored_hashes = load_stored_hashes(resources_dir).unwrap_or_default();
    let damaged = DEFAULT_FILTERS
        .iter()
        .filter(|path| {
            let local = resources_dir.join(path);
            !local.is_file()
                || stored_hashes
                    .get(**path)
                    .is_none_or(|expected| hash_differs(&local, expected))
        })
        .copied()
        .collect::<Vec<_>>();
    if damaged.is_empty() {
        return Ok(Vec::new());
    }

    let remote_hashes = fetch_remote_hashes(client).await?;
    for rel_path in &damaged {
        let expected_hash = remote_hashes
            .get(*rel_path)
            .with_context(|| format!("remote hashes.json does not contain {rel_path}"))?;
        download_verified_file(client, resources_dir, rel_path, expected_hash).await?;
        stored_hashes.insert((*rel_path).to_owned(), expected_hash.clone());
    }
    write_hashes(resources_dir, &stored_hashes)?;
    Ok(damaged.into_iter().map(str::to_owned).collect())
}

pub async fn check_remote_updates(
    client: &Client,
    resources_dir: &Path,
) -> Result<AppHealthSnapshot> {
    let mut snapshot = check_local_health(resources_dir);
    let remote_hashes = fetch_remote_hashes(client).await?;
    let mut available_updates = Vec::new();
    for (rel_path, expected_hash) in remote_hashes {
        if !is_core_path(&rel_path) {
            continue;
        }
        let local_file = resources_dir.join(&rel_path);
        if !local_file.is_file() || hash_differs(&local_file, &expected_hash) {
            available_updates.push(rel_path);
        }
    }

    snapshot.available_updates = available_updates;
    Ok(snapshot)
}

fn is_core_path(path: &str) -> bool {
    BINARIES.contains(&path) || path.starts_with("fake/") || path.starts_with("modules/")
}

pub async fn repair_managed_files(client: &Client, resources_dir: &Path) -> Result<Vec<String>> {
    let (mut stored_hashes, trusted_manifest) = match load_stored_hashes(resources_dir) {
        Ok(hashes) if !hashes.is_empty() => (hashes, true),
        Ok(_) | Err(_) => (HashMap::new(), false),
    };
    let mut remote_hashes = if trusted_manifest {
        None
    } else {
        Some(fetch_remote_hashes(client).await?)
    };
    let mut restored = Vec::new();

    for rel_path in tracked_paths() {
        let expected = if trusted_manifest {
            stored_hashes.get(&rel_path)
        } else {
            remote_hashes
                .as_ref()
                .and_then(|hashes| hashes.get(&rel_path))
        };
        let local = resources_dir.join(&rel_path);
        if local.is_file() && expected.is_some_and(|hash| !hash_differs(&local, hash)) {
            continue;
        }

        if remote_hashes.is_none() {
            remote_hashes = Some(fetch_remote_hashes(client).await?);
        }
        let expected_hash = remote_hashes
            .as_ref()
            .and_then(|hashes| hashes.get(&rel_path))
            .with_context(|| format!("remote hashes.json does not contain {rel_path}"))?
            .clone();
        download_verified_file(client, resources_dir, &rel_path, &expected_hash).await?;
        stored_hashes.insert(rel_path.clone(), expected_hash);
        restored.push(rel_path);
    }

    if !trusted_manifest {
        stored_hashes.extend(remote_hashes.unwrap_or_default());
    }
    if !restored.is_empty() || !trusted_manifest {
        write_hashes(resources_dir, &stored_hashes)?;
    }
    Ok(restored)
}

pub async fn download_missing_or_outdated_files(
    client: &Client,
    resources_dir: &Path,
    mut on_progress: impl FnMut(DownloadProgress) + Send + 'static,
) -> Result<()> {
    let remote_hashes = fetch_remote_hashes(client).await?;
    let stored_hashes = load_stored_hashes(resources_dir)?;

    let mut needed = Vec::new();
    for rel_path in tracked_paths() {
        let expected_hash = remote_hashes
            .get(&rel_path)
            .or_else(|| stored_hashes.get(&rel_path))
            .with_context(|| format!("remote hashes.json does not contain {rel_path}"))?;
        let local = resources_dir.join(&rel_path);
        if !local.is_file() || hash_differs(&local, expected_hash) {
            needed.push(rel_path);
        }
    }

    let total = needed.len();
    for (idx, rel_path) in needed.into_iter().enumerate() {
        on_progress(DownloadProgress {
            current: idx + 1,
            total,
            filename: rel_path.clone(),
        });

        let url = format!("{THIRD_PARTY_BASE_URL}/{rel_path}");
        let dest = resources_dir.join(&rel_path);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }

        let resp = client
            .get(&url)
            .send()
            .await
            .with_context(|| format!("failed to download {url}"))?;
        if !resp.status().is_success() {
            bail!("downloading {url} returned HTTP {}", resp.status());
        }

        let bytes = resp.bytes().await?;
        let expected_hash = remote_hashes
            .get(&rel_path)
            .or_else(|| stored_hashes.get(&rel_path))
            .context("download plan lost its expected hash")?;
        let actual_hash = compute_sha256_bytes(&bytes);
        if !actual_hash.eq_ignore_ascii_case(expected_hash) {
            bail!("hash mismatch for {rel_path}: expected {expected_hash}, got {actual_hash}");
        }
        let tmp = dest.with_extension("tmp");
        fs::write(&tmp, bytes).with_context(|| format!("failed to write {}", tmp.display()))?;
        replace_file(&tmp, &dest)?;
    }

    let mut next_hashes = stored_hashes;
    next_hashes.extend(remote_hashes);
    write_hashes(resources_dir, &next_hashes)?;

    Ok(())
}

pub async fn refresh_stale_lists(client: &Client, resources_dir: &Path) -> Result<Vec<String>> {
    let remote_hashes = fetch_remote_hashes(client).await?;
    let mut stored_hashes = load_stored_hashes(resources_dir)?;
    let mut updated = Vec::new();
    for rel_path in LISTS {
        let url = format!("{THIRD_PARTY_BASE_URL}/{rel_path}");
        let dest = resources_dir.join(rel_path);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }

        let expected_hash = remote_hashes
            .get(*rel_path)
            .with_context(|| format!("remote hashes.json does not contain {rel_path}"))?;
        if dest.is_file()
            && compute_sha256(&dest).is_ok_and(|hash| hash.eq_ignore_ascii_case(expected_hash))
        {
            continue;
        }
        let resp = client
            .get(&url)
            .send()
            .await
            .with_context(|| format!("failed to download {url}"))?;
        if !resp.status().is_success() {
            bail!("downloading {url} returned HTTP {}", resp.status());
        }
        let bytes = resp.bytes().await?;
        let actual_hash = compute_sha256_bytes(&bytes);
        if !actual_hash.eq_ignore_ascii_case(expected_hash) {
            bail!("hash mismatch for {rel_path}: expected {expected_hash}, got {actual_hash}");
        }
        let tmp = dest.with_extension("tmp");
        fs::write(&tmp, bytes).with_context(|| format!("failed to write {}", tmp.display()))?;
        replace_file(&tmp, &dest)?;
        updated.push((*rel_path).to_string());
    }
    stored_hashes.extend(remote_hashes);
    write_hashes(resources_dir, &stored_hashes)?;
    Ok(updated)
}

fn write_hashes(resources_dir: &Path, hashes: &HashMap<String, String>) -> Result<()> {
    let path = resources_dir.join("hashes.json");
    let temporary = path.with_extension("json.tmp");
    let compatible_hashes = hashes
        .iter()
        .map(|(path, hash)| (manifest_key(path), hash))
        .collect::<HashMap<_, _>>();
    let json = serde_json::to_vec_pretty(&compatible_hashes)?;
    fs::write(&temporary, json)
        .with_context(|| format!("failed to write {}", temporary.display()))?;
    replace_file(&temporary, &path)
}

fn manifest_key(path: &str) -> String {
    for group in ["fake", "lists", "modules"] {
        if let Some(name) = path.strip_prefix(&format!("{group}/")) {
            return format!("{group}:{name}");
        }
    }
    if BINARIES.contains(&path) {
        format!("binaries:{path}")
    } else {
        path.to_owned()
    }
}

fn replace_file(temporary: &Path, destination: &Path) -> Result<()> {
    if !destination.exists() {
        return fs::rename(temporary, destination).with_context(|| {
            format!(
                "failed to move {} to {}",
                temporary.display(),
                destination.display()
            )
        });
    }
    let backup = destination.with_extension("bak");
    if backup.exists() {
        fs::remove_file(&backup)
            .with_context(|| format!("failed to remove {}", backup.display()))?;
    }
    fs::rename(destination, &backup)
        .with_context(|| format!("failed to back up {}", destination.display()))?;
    if let Err(error) = fs::rename(temporary, destination) {
        fs::rename(&backup, destination).with_context(|| {
            format!(
                "failed to install {} ({error}) and restore {}",
                destination.display(),
                backup.display()
            )
        })?;
        bail!("failed to install {}: {error}", destination.display());
    }
    fs::remove_file(&backup).with_context(|| format!("failed to remove {}", backup.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grouped_manifest_keys_map_to_portable_paths() {
        let hashes = normalize_remote_hashes(HashMap::from([
            ("binaries:winws.exe".to_owned(), "a".to_owned()),
            ("fake:stun.bin".to_owned(), "b".to_owned()),
            (
                "modules:dnscrypt-proxy/dnscrypt-proxy.exe".to_owned(),
                "c".to_owned(),
            ),
        ]));

        assert_eq!(hashes.get("winws.exe").map(String::as_str), Some("a"));
        assert_eq!(hashes.get("fake/stun.bin").map(String::as_str), Some("b"));
        assert_eq!(
            hashes
                .get("modules/dnscrypt-proxy/dnscrypt-proxy.exe")
                .map(String::as_str),
            Some("c")
        );
    }

    #[test]
    fn test_compute_sha256() {
        let temp_dir =
            std::env::temp_dir().join(format!("zapret_test_{}", uuid::Uuid::new_v4().simple()));
        fs::create_dir_all(&temp_dir).unwrap();
        let test_file = temp_dir.join("test.txt");
        fs::write(&test_file, b"hello world\n").unwrap();

        let hash = compute_sha256(&test_file).unwrap();
        assert_eq!(
            hash,
            "a948904f2f0f479b8f8197694b30184b0d2ed1c1cd2a1ec0fb85d299a192a447"
        );

        let _cleanup_result = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_check_local_health_missing_files() {
        let temp_dir =
            std::env::temp_dir().join(format!("zapret_test_{}", uuid::Uuid::new_v4().simple()));
        fs::create_dir_all(&temp_dir).unwrap();

        let health = check_local_health(&temp_dir);
        assert!(!health.binaries_ok);
        assert!(!health.missing_critical_files.is_empty());
        assert!(
            health
                .missing_critical_files
                .contains(&"WinDivert.dll".to_string())
        );

        let _cleanup_result = fs::remove_dir_all(&temp_dir);
    }
}
