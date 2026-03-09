use crate::commands::process::kill_windivert_service;
use crate::config::{get_config_path, get_zapret_dir};
use futures::stream::{self, StreamExt};
use notify::{Config as NotifyConfig, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;
use tokio::fs as tokio_fs;
use tokio::time::sleep;

#[derive(Clone, serde::Serialize)]
struct DownloadProgress {
    current: usize,
    total: usize,
    filename: String,
    phase: String,
}

#[derive(Clone, serde::Serialize)]
pub struct FileHealthChangedPayload {
    pub binaries_ok: bool,
    pub lists_changed: bool,
    pub config_missing: bool,
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
struct ListsState {
    last_updated_at: Option<u64>,
}

const BINARIES: [(&str, &str); 4] = [
    (
        "WinDivert.dll",
        "https://github.com/bol-van/zapret-win-bundle/raw/refs/heads/master/zapret-winws/WinDivert.dll",
    ),
    (
        "WinDivert64.sys",
        "https://github.com/bol-van/zapret-win-bundle/raw/refs/heads/master/zapret-winws/WinDivert64.sys",
    ),
    (
        "winws.exe",
        "https://github.com/bol-van/zapret-win-bundle/raw/refs/heads/master/zapret-winws/winws.exe",
    ),
    (
        "cygwin1.dll",
        "https://github.com/bol-van/zapret-win-bundle/raw/refs/heads/master/zapret-winws/cygwin1.dll",
    ),
];

const FAKE_FILES_BASE_URL: &str = "https://raw.githubusercontent.com/Noktomezo/ZIStorage/main/fake";
const FAKE_FILES: &[&str] = &[
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
];

const LISTS_BASE_URL: &str = "https://raw.githubusercontent.com/Noktomezo/ZIStorage/main/lists";
const LISTS: &[&str] = &[
    "zapret-hosts-google.txt",
    "zapret-hosts-user-exclude.txt",
    "zapret-ip-user.txt",
];

const FILTERS_BASE_URL: &str = "https://raw.githubusercontent.com/bol-van/zapret-win-bundle/master/zapret-winws/windivert.filter";
const FILTERS: &[&str] = &[
    "windivert_part.dht.txt",
    "windivert_part.discord_media.txt",
    "windivert_part.quic_initial_ietf.txt",
    "windivert_part.stun.txt",
    "windivert_part.wireguard.txt",
];

const LISTS_REFRESH_INTERVAL_SECS: u64 = 60 * 60 * 3;
const WATCH_DEBOUNCE_MS: u64 = 800;
const FILES_CONCURRENCY_LIMIT: usize = 6;
static FILES_WATCHER_STARTED: AtomicBool = AtomicBool::new(false);
static HASHES_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

struct FileToDownload {
    name: String,
    url: String,
    dest_path: PathBuf,
    hash_key: Option<String>,
    phase: String,
    cached_bytes: Option<Vec<u8>>,
}

fn get_fake_dir() -> PathBuf {
    get_zapret_dir().join("fake")
}
fn get_lists_dir() -> PathBuf {
    get_zapret_dir().join("lists")
}
fn get_filters_dir() -> PathBuf {
    get_zapret_dir().join("filters")
}
fn get_hashes_path() -> PathBuf {
    get_zapret_dir().join("hashes.json")
}
fn get_lists_state_path() -> PathBuf {
    get_zapret_dir().join("lists-state.json")
}

fn sanitize_filename(filename: &str) -> Result<String, String> {
    if filename.is_empty() {
        return Err("Filename cannot be empty".to_string());
    }
    if filename.contains('/') || filename.contains('\\') {
        return Err("Path separators not allowed in filename".to_string());
    }

    let name = Path::new(filename)
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "Invalid filename".to_string())?;

    if filename != name {
        return Err("Path separators not allowed in filename".to_string());
    }
    if name.is_empty() {
        return Err("Filename cannot be empty".to_string());
    }
    if name == "." || name == ".." {
        return Err("Invalid filename".to_string());
    }

    let valid_chars = name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_');
    if !valid_chars {
        return Err("Filename contains invalid characters".to_string());
    }

    let file_stem = Path::new(name)
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase());
    if let Some(stem) = file_stem {
        const RESERVED_NAMES: &[&str] = &[
            "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7",
            "com8", "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
        ];
        if RESERVED_NAMES.contains(&stem.as_str()) {
            return Err("Filename uses reserved name".to_string());
        }
    }

    Ok(name.to_string())
}

fn calculate_sha256(file_path: &PathBuf) -> Result<String, String> {
    let mut hasher = Sha256::new();
    let mut file = fs::File::open(file_path).map_err(|e| e.to_string())?;
    std::io::copy(&mut file, &mut hasher).map_err(|e| e.to_string())?;
    Ok(format!("{:x}", hasher.finalize()))
}

struct RemoteCheckTarget {
    name: String,
    url: String,
    dest_path: PathBuf,
}

fn calculate_sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn load_stored_hashes() -> Result<HashMap<String, String>, String> {
    let hashes_path = get_hashes_path();
    if !hashes_path.exists() {
        return Ok(HashMap::new());
    }
    let content =
        fs::read_to_string(&hashes_path).map_err(|e| format!("Failed to read hashes.json: {e}"))?;
    serde_json::from_str(&content).map_err(|e| format!("Failed to parse hashes.json: {e}"))
}

fn save_stored_hashes(hashes: &HashMap<String, String>) -> Result<(), String> {
    let content = serde_json::to_string_pretty(hashes).map_err(|e| e.to_string())?;
    let hashes_path = get_hashes_path();
    let temp_path = hashes_path.with_extension("json.tmp");
    let mut temp_file = fs::File::create(&temp_path).map_err(|e| e.to_string())?;
    use std::io::Write;
    temp_file
        .write_all(content.as_bytes())
        .map_err(|e| e.to_string())?;
    temp_file.sync_all().map_err(|e| e.to_string())?;
    fs::rename(temp_path, hashes_path).map_err(|e| e.to_string())?;
    Ok(())
}

fn load_lists_state() -> Result<ListsState, String> {
    let path = get_lists_state_path();
    if !path.exists() {
        return Ok(ListsState::default());
    }
    let content =
        fs::read_to_string(&path).map_err(|e| format!("Failed to read lists-state.json: {e}"))?;
    serde_json::from_str(&content).map_err(|e| format!("Failed to parse lists-state.json: {e}"))
}

fn save_lists_state(state: &ListsState) -> Result<(), String> {
    let content = serde_json::to_string_pretty(state).map_err(|e| e.to_string())?;
    let state_path = get_lists_state_path();
    let temp_path = state_path.with_extension("json.tmp");
    let mut temp_file = fs::File::create(&temp_path).map_err(|e| e.to_string())?;
    use std::io::Write;
    temp_file
        .write_all(content.as_bytes())
        .map_err(|e| e.to_string())?;
    temp_file.sync_all().map_err(|e| e.to_string())?;
    fs::rename(temp_path, state_path).map_err(|e| e.to_string())
}

fn rebuild_lists_state() -> Result<(), String> {
    save_lists_state(&ListsState {
        last_updated_at: None,
    })
}

fn update_hashes<F>(mutate: F) -> Result<(), String>
where
    F: FnOnce(&mut HashMap<String, String>) -> Result<(), String>,
{
    let _guard = HASHES_LOCK
        .lock()
        .map_err(|e| format!("Failed to lock hashes.json: {e}"))?;
    let mut hashes = load_stored_hashes()?;
    mutate(&mut hashes)?;
    save_stored_hashes(&hashes)
}

fn rebuild_hashes_from_disk() -> Result<(), String> {
    let _guard = HASHES_LOCK
        .lock()
        .map_err(|e| format!("Failed to lock hashes.json: {e}"))?;
    let mut hashes = HashMap::new();

    for name in binary_names() {
        let path = get_zapret_dir().join(name);
        if path.exists() {
            hashes.insert(hash_key("binaries", name), calculate_sha256(&path)?);
        }
    }

    for name in FAKE_FILES {
        let path = get_fake_dir().join(name);
        if path.exists() {
            hashes.insert(hash_key("fake", name), calculate_sha256(&path)?);
        }
    }

    for name in FILTERS {
        let path = get_filters_dir().join(name);
        if path.exists() {
            hashes.insert(hash_key("filters", name), calculate_sha256(&path)?);
        }
    }

    for name in LISTS {
        let path = get_lists_dir().join(name);
        if path.exists() {
            hashes.insert(hash_key("lists", name), calculate_sha256(&path)?);
        }
    }

    save_stored_hashes(&hashes)
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn create_http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent("Mozilla/5.0")
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())
}

fn hash_key(group: &str, name: &str) -> String {
    format!("{group}:{name}")
}

fn expected_hash<'a>(
    hashes: &'a HashMap<String, String>,
    group: &str,
    name: &str,
) -> Option<&'a String> {
    hashes.get(&hash_key(group, name)).or_else(|| {
        if group == "binaries" {
            hashes.get(name)
        } else {
            None
        }
    })
}

fn file_needs_download(
    file_path: &Path,
    group: &str,
    name: &str,
    hashes: &HashMap<String, String>,
    requires_hash: bool,
) -> Result<bool, String> {
    if !file_path.exists() {
        return Ok(true);
    }

    if !requires_hash {
        return Ok(false);
    }

    let Some(expected) = expected_hash(hashes, group, name) else {
        return Ok(true);
    };

    let actual = calculate_sha256(&file_path.to_path_buf())?;
    Ok(actual != *expected)
}

fn verify_group(
    base_dir: &Path,
    group: &str,
    names: &[&str],
    hashes: &HashMap<String, String>,
) -> Result<bool, String> {
    for name in names {
        let file_path = base_dir.join(name);
        if !file_path.exists() {
            return Ok(false);
        }
        let Some(expected) = expected_hash(hashes, group, name) else {
            return Ok(false);
        };
        let actual = calculate_sha256(&file_path)?;
        if actual != *expected {
            return Ok(false);
        }
    }
    Ok(true)
}

fn ensure_base_directories() -> Result<(), String> {
    for dir in [
        get_zapret_dir(),
        get_fake_dir(),
        get_lists_dir(),
        get_filters_dir(),
    ] {
        if !dir.exists() {
            fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn ensure_helper_files() -> Result<(), String> {
    ensure_base_directories()?;
    if !get_lists_state_path().exists() {
        rebuild_lists_state()?;
    }
    Ok(())
}

fn binary_names() -> Vec<&'static str> {
    BINARIES.iter().map(|(name, _)| *name).collect()
}

fn critical_files_ok() -> Result<bool, String> {
    ensure_helper_files()?;
    let stored_hashes = load_stored_hashes()?;
    if !verify_group(
        &get_zapret_dir(),
        "binaries",
        &binary_names(),
        &stored_hashes,
    )? {
        return Ok(false);
    }
    if !verify_group(&get_fake_dir(), "fake", FAKE_FILES, &stored_hashes)? {
        return Ok(false);
    }
    if !verify_group(&get_lists_dir(), "lists", LISTS, &stored_hashes)? {
        return Ok(false);
    }
    Ok(true)
}

fn collect_missing_critical_files() -> Result<Vec<String>, String> {
    ensure_helper_files()?;
    let stored_hashes = load_stored_hashes()?;
    let mut missing = Vec::new();

    for name in binary_names() {
        if file_needs_download(
            &get_zapret_dir().join(name),
            "binaries",
            name,
            &stored_hashes,
            true,
        )? {
            missing.push(name.to_string());
        }
    }

    for name in FAKE_FILES {
        if file_needs_download(
            &get_fake_dir().join(name),
            "fake",
            name,
            &stored_hashes,
            true,
        )? {
            missing.push((*name).to_string());
        }
    }

    for name in LISTS {
        if file_needs_download(
            &get_lists_dir().join(name),
            "lists",
            name,
            &stored_hashes,
            true,
        )? {
            missing.push((*name).to_string());
        }
    }

    Ok(missing)
}

async fn check_remote_file(
    file_path: PathBuf,
    client: &reqwest::Client,
    url: &str,
    name: &str,
) -> Result<(bool, Option<Vec<u8>>), String> {
    if !file_path.exists() {
        return Ok((true, None));
    }

    let local_hash = calculate_sha256_async(file_path).await?;
    let bytes = download_bytes(client, url, name).await?;
    let remote_hash = calculate_sha256_bytes(&bytes);
    let changed = local_hash != remote_hash;
    Ok((changed, changed.then_some(bytes)))
}

async fn collect_available_updates() -> Result<Vec<String>, String> {
    ensure_helper_files()?;
    let client = create_http_client()?;
    let stored_hashes = load_stored_hashes()?;
    let mut targets = Vec::new();

    for (name, url) in BINARIES {
        let path = get_zapret_dir().join(name);
        if !file_needs_download(&path, "binaries", name, &stored_hashes, true)? {
            targets.push(RemoteCheckTarget {
                name: name.to_string(),
                url: url.to_string(),
                dest_path: path,
            });
        }
    }

    for name in FAKE_FILES {
        let path = get_fake_dir().join(name);
        if !file_needs_download(&path, "fake", name, &stored_hashes, true)? {
            targets.push(RemoteCheckTarget {
                name: (*name).to_string(),
                url: format!("{FAKE_FILES_BASE_URL}/{name}"),
                dest_path: path,
            });
        }
    }

    let client = &client;
    let results = stream::iter(targets.into_iter().map(|target| async move {
        let (changed, _) =
            check_remote_file(target.dest_path, client, &target.url, &target.name).await?;
        Ok::<Option<String>, String>(if changed { Some(target.name) } else { None })
    }))
    .buffer_unordered(FILES_CONCURRENCY_LIMIT)
    .collect::<Vec<_>>()
    .await;

    let mut updates = Vec::new();
    for result in results {
        if let Some(name) = result? {
            updates.push(name);
        }
    }

    updates.sort();
    Ok(updates)
}

fn path_is_inside(path: &Path, dir: &Path) -> bool {
    path.starts_with(dir)
}

fn event_affects_lists(paths: &[PathBuf]) -> bool {
    let lists_dir = get_lists_dir();
    paths.iter().any(|path| path_is_inside(path, &lists_dir))
}

fn event_affects_tracked_files(paths: &[PathBuf]) -> bool {
    let base_dir = get_zapret_dir();
    let fake_dir = get_fake_dir();
    let hashes_path = get_hashes_path();
    let lists_state_path = get_lists_state_path();
    let config_path = get_config_path();
    let critical_binary_names = binary_names();
    paths.iter().any(|path| {
        path_is_inside(path, &fake_dir)
            || path == &hashes_path
            || path == &lists_state_path
            || path == &config_path
            || critical_binary_names
                .iter()
                .any(|name| path == &base_dir.join(name))
    })
}

async fn calculate_sha256_async(file_path: PathBuf) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || calculate_sha256(&file_path))
        .await
        .map_err(|e| format!("Failed to join SHA-256 task: {e}"))?
}

pub fn start_files_watcher(app: AppHandle) -> Result<(), String> {
    ensure_base_directories()?;
    if FILES_WATCHER_STARTED.swap(true, Ordering::SeqCst) {
        return Ok(());
    }
    let watch_dir = get_zapret_dir();

    std::thread::spawn(move || {
        let (tx, rx) = mpsc::channel::<notify::Result<notify::Event>>();
        let mut watcher = match RecommendedWatcher::new(
            move |result| {
                let _ = tx.send(result);
            },
            NotifyConfig::default(),
        ) {
            Ok(watcher) => watcher,
            Err(e) => {
                FILES_WATCHER_STARTED.store(false, Ordering::SeqCst);
                let _ = app.emit(
                    "files-health-watch-error",
                    format!("Не удалось запустить watcher файлов: {e}"),
                );
                return;
            }
        };

        if let Err(e) = watcher.watch(&watch_dir, RecursiveMode::Recursive) {
            FILES_WATCHER_STARTED.store(false, Ordering::SeqCst);
            let _ = app.emit(
                "files-health-watch-error",
                format!("Не удалось подписаться на каталог файлов: {e}"),
            );
            return;
        }

        loop {
            let event = match rx.recv() {
                Ok(Ok(event)) => event,
                Ok(Err(e)) => {
                    let _ = app.emit(
                        "files-health-watch-error",
                        format!("Ошибка watcher файлов: {e}"),
                    );
                    continue;
                }
                Err(_) => break,
            };

            let mut batch = vec![event];
            while let Ok(next) = rx.recv_timeout(Duration::from_millis(WATCH_DEBOUNCE_MS)) {
                match next {
                    Ok(next_event) => batch.push(next_event),
                    Err(e) => {
                        let _ = app.emit(
                            "files-health-watch-error",
                            format!("Ошибка watcher файлов: {e}"),
                        );
                    }
                }
            }

            let relevant_events: Vec<_> = batch
                .into_iter()
                .filter(|event| !matches!(event.kind, EventKind::Access(_)))
                .collect();

            if relevant_events.is_empty() {
                continue;
            }

            let mut changed_paths = Vec::new();
            for event in relevant_events {
                changed_paths.extend(event.paths);
            }

            let tracked_files_changed = event_affects_tracked_files(&changed_paths);
            let lists_changed = event_affects_lists(&changed_paths);
            let lists_state_changed = changed_paths
                .iter()
                .any(|path| path == &get_lists_state_path());

            if !tracked_files_changed && !lists_changed {
                continue;
            }

            if lists_state_changed
                && !get_lists_state_path().exists()
                && let Err(e) = rebuild_lists_state()
            {
                let _ = app.emit(
                    "files-health-watch-error",
                    format!("Не удалось пересоздать lists-state.json: {e}"),
                );
            }

            let binaries_ok = match critical_files_ok() {
                Ok(ok) => ok,
                Err(e) => {
                    let _ = app.emit(
                        "files-health-watch-error",
                        format!("Не удалось перепроверить файлы: {e}"),
                    );
                    continue;
                }
            };

            let _ = app.emit(
                "files-health-changed",
                FileHealthChangedPayload {
                    binaries_ok,
                    lists_changed,
                    config_missing: !get_config_path().exists(),
                },
            );
        }
    });

    Ok(())
}

async fn download_bytes(
    client: &reqwest::Client,
    url: &str,
    name: &str,
) -> Result<Vec<u8>, String> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch {name}: {e}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "Failed to download {name}: HTTP {}",
            response.status()
        ));
    }
    response
        .bytes()
        .await
        .map(|bytes| bytes.to_vec())
        .map_err(|e| format!("Failed to read {name} body: {e}"))
}

async fn refresh_lists_internal(force: bool) -> Result<usize, String> {
    ensure_helper_files()?;

    let lists_dir = get_lists_dir();
    let stored_hashes = load_stored_hashes()?;
    let state = load_lists_state()
        .inspect_err(|e| eprintln!("Failed to load lists-state.json, using defaults: {e}"))
        .unwrap_or_default();
    let now = current_timestamp();
    let all_lists_exist = verify_group(&lists_dir, "lists", LISTS, &stored_hashes).unwrap_or(false);
    let is_stale = state
        .last_updated_at
        .map(|last| now.saturating_sub(last) >= LISTS_REFRESH_INTERVAL_SECS)
        .unwrap_or(true);

    if !force && all_lists_exist && !is_stale {
        return Ok(0);
    }

    let client = create_http_client()?;
    let mut updated_count = 0usize;
    for name in LISTS {
        let bytes = download_bytes(&client, &format!("{LISTS_BASE_URL}/{name}"), name).await?;
        let remote_hash = calculate_sha256_bytes(&bytes);
        let file_path = lists_dir.join(name);
        let local_hash = if file_path.exists() {
            calculate_sha256(&file_path).ok()
        } else {
            None
        };

        if local_hash.as_deref() != Some(remote_hash.as_str()) {
            tokio_fs::write(&file_path, &bytes)
                .await
                .map_err(|e| format!("Failed to write {name}: {e}"))?;
            update_hashes(|hashes| {
                hashes.insert(hash_key("lists", name), remote_hash.clone());
                Ok(())
            })?;
            updated_count += 1;
        } else {
            update_hashes(|hashes| {
                hashes.insert(hash_key("lists", name), remote_hash.clone());
                Ok(())
            })?;
        }
    }

    save_lists_state(&ListsState {
        last_updated_at: Some(now),
    })?;
    Ok(updated_count)
}

pub async fn restore_default_filters_internal() -> Result<(), String> {
    ensure_base_directories()?;
    let client = create_http_client()?;
    let filters_dir = get_filters_dir();

    for name in FILTERS {
        let dest_path = filters_dir.join(name);
        let bytes = download_bytes(&client, &format!("{FILTERS_BASE_URL}/{name}"), name).await?;
        tokio_fs::write(&dest_path, &bytes)
            .await
            .map_err(|e| format!("Failed to write {name}: {e}"))?;

        let hash = calculate_sha256_async(dest_path).await?;
        update_hashes(|hashes| {
            hashes.insert(hash_key("filters", name), hash);
            Ok(())
        })?;
    }

    Ok(())
}

#[tauri::command]
pub fn verify_binaries() -> Result<bool, String> {
    critical_files_ok()
}

#[tauri::command]
pub fn get_missing_critical_files() -> Result<Vec<String>, String> {
    collect_missing_critical_files()
}

#[tauri::command]
pub async fn get_available_updates() -> Result<Vec<String>, String> {
    collect_available_updates().await
}

#[tauri::command]
pub fn restore_hashes_from_disk() -> Result<(), String> {
    rebuild_hashes_from_disk()
}

#[tauri::command]
pub async fn download_binaries(app: AppHandle, force_all: Option<bool>) -> Result<(), String> {
    ensure_helper_files()?;
    let force_all = force_all.unwrap_or(false);

    let dir = get_zapret_dir();
    let fake_dir = get_fake_dir();
    let lists_dir = get_lists_dir();
    let filters_dir = get_filters_dir();
    let client = create_http_client()?;
    let mut candidates: Vec<FileToDownload> = vec![];

    for (name, url) in BINARIES {
        candidates.push(FileToDownload {
            name: name.to_string(),
            url: url.to_string(),
            dest_path: dir.join(name),
            hash_key: Some(hash_key("binaries", name)),
            phase: "binaries".to_string(),
            cached_bytes: None,
        });
    }
    for name in FAKE_FILES {
        candidates.push(FileToDownload {
            name: name.to_string(),
            url: format!("{FAKE_FILES_BASE_URL}/{name}"),
            dest_path: fake_dir.join(name),
            hash_key: Some(hash_key("fake", name)),
            phase: "fake".to_string(),
            cached_bytes: None,
        });
    }
    for name in FILTERS {
        candidates.push(FileToDownload {
            name: name.to_string(),
            url: format!("{FILTERS_BASE_URL}/{name}"),
            dest_path: filters_dir.join(name),
            hash_key: Some(hash_key("filters", name)),
            phase: "filters".to_string(),
            cached_bytes: None,
        });
    }
    for name in LISTS {
        candidates.push(FileToDownload {
            name: name.to_string(),
            url: format!("{LISTS_BASE_URL}/{name}"),
            dest_path: lists_dir.join(name),
            hash_key: Some(hash_key("lists", name)),
            phase: "lists".to_string(),
            cached_bytes: None,
        });
    }

    let files_to_download = if force_all {
        candidates
    } else {
        let client = &client;
        let stored_hashes = Arc::new(load_stored_hashes()?);
        let results = stream::iter(candidates.into_iter().map(|file| {
            let stored_hashes = stored_hashes.clone();
            async move {
                let checked = if file.dest_path.exists() {
                    let (needs_download, cached_bytes) =
                        check_remote_file(file.dest_path.clone(), client, &file.url, &file.name)
                            .await?;
                    (needs_download, cached_bytes)
                } else {
                    (true, None)
                };

                let (needs_download, cached_bytes) = checked;
                let hash_missing = file
                    .hash_key
                    .as_ref()
                    .map(|key| !stored_hashes.contains_key(key))
                    .unwrap_or(false);
                Ok::<Option<FileToDownload>, String>(
                    if needs_download || cached_bytes.is_some() || hash_missing {
                        Some(FileToDownload {
                            cached_bytes,
                            ..file
                        })
                    } else {
                        None
                    },
                )
            }
        }))
        .buffer_unordered(FILES_CONCURRENCY_LIMIT)
        .collect::<Vec<_>>()
        .await;

        let mut filtered = Vec::new();
        for result in results {
            if let Some(file) = result? {
                filtered.push(file);
            }
        }
        filtered
    };

    let total_files = files_to_download.len();
    app.emit("download-start", total_files).ok();

    if total_files == 0 {
        app.emit("download-complete", ()).ok();
        let _ = app
            .notification()
            .builder()
            .title("Готово")
            .body("Все файлы уже актуальны")
            .show();
        return Ok(());
    }

    #[cfg(windows)]
    {
        if let Err(e) = kill_windivert_service() {
            if e.contains("does not exist") || e.contains("marked for deletion") {
                eprintln!("Non-fatal WinDivert service state before download: {e}");
            } else {
                let err = format!("Failed to stop WinDivert service before download: {e}");
                app.emit("download-error", err.clone()).ok();
                return Err(err);
            }
        }
        sleep(Duration::from_millis(300)).await;
    }

    let downloaded_lists = files_to_download.iter().any(|file| file.phase == "lists");

    for (current, file) in files_to_download.iter().enumerate() {
        app.emit(
            "download-progress",
            DownloadProgress {
                current: current + 1,
                total: total_files,
                filename: file.name.clone(),
                phase: file.phase.clone(),
            },
        )
        .ok();

        let bytes = if let Some(bytes) = &file.cached_bytes {
            bytes.clone()
        } else {
            match download_bytes(&client, &file.url, &file.name).await {
                Ok(bytes) => bytes,
                Err(err) => {
                    app.emit("download-error", err.clone()).ok();
                    return Err(err);
                }
            }
        };

        if let Err(e) = tokio_fs::write(&file.dest_path, &bytes).await {
            let err = format!("Failed to write {}: {}", file.name, e);
            app.emit("download-error", err.clone()).ok();
            return Err(err);
        }

        if let Some(hash_key) = &file.hash_key {
            let hash = calculate_sha256_async(file.dest_path.clone()).await?;
            update_hashes(|hashes| {
                hashes.insert(hash_key.clone(), hash);
                Ok(())
            })?;
        }
    }

    if downloaded_lists {
        save_lists_state(&ListsState {
            last_updated_at: Some(current_timestamp()),
        })?;
    }

    app.emit("download-complete", ()).ok();
    let _ = app
        .notification()
        .builder()
        .title("Готово")
        .body(format!("Обновлено {} файлов приложения", total_files))
        .show();
    Ok(())
}

#[tauri::command]
pub async fn refresh_lists_if_stale() -> Result<usize, String> {
    refresh_lists_internal(false).await
}
#[tauri::command]
pub async fn restore_default_filters() -> Result<(), String> {
    restore_default_filters_internal().await
}
#[tauri::command]
pub fn get_binary_path(filename: &str) -> Result<String, String> {
    let filename = sanitize_filename(filename)?;
    if !BINARIES.iter().any(|(name, _)| *name == filename) {
        return Err("Unknown binary filename".to_string());
    }
    Ok(get_zapret_dir()
        .join(filename)
        .to_string_lossy()
        .to_string())
}
#[tauri::command]
pub fn get_winws_path() -> String {
    get_zapret_dir()
        .join("winws.exe")
        .to_string_lossy()
        .to_string()
}
#[tauri::command]
pub fn get_filters_path() -> String {
    get_filters_dir().to_string_lossy().to_string()
}
#[tauri::command]
pub fn get_reserved_filter_filenames() -> Vec<String> {
    FILTERS.iter().map(|name| (*name).to_string()).collect()
}

#[tauri::command]
pub fn save_filter_file(filename: String, content: String) -> Result<(), String> {
    let filename = sanitize_filename(&filename)?;
    let filters_dir = get_filters_dir();
    if !filters_dir.exists() {
        fs::create_dir_all(&filters_dir).map_err(|e| e.to_string())?;
    }

    let file_path = filters_dir.join(&filename);
    fs::write(&file_path, content).map_err(|e| e.to_string())?;

    if FILTERS.contains(&filename.as_str()) {
        let hash = calculate_sha256(&file_path)?;
        update_hashes(|hashes| {
            hashes.insert(hash_key("filters", &filename), hash);
            Ok(())
        })?;
    }

    Ok(())
}

#[tauri::command]
pub fn load_filter_file(filename: String) -> Result<String, String> {
    let filename = sanitize_filename(&filename)?;
    fs::read_to_string(get_filters_dir().join(&filename)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_filter_file(filename: String) -> Result<(), String> {
    let filename = sanitize_filename(&filename)?;
    let file_path = get_filters_dir().join(&filename);
    if file_path.exists() {
        fs::remove_file(file_path).map_err(|e| e.to_string())?;
    }
    update_hashes(|hashes| {
        hashes.remove(&hash_key("filters", &filename));
        Ok(())
    })?;
    Ok(())
}

#[tauri::command]
pub fn open_zapret_directory(app: AppHandle) -> Result<(), String> {
    let dir = get_zapret_dir();
    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    app.opener()
        .open_path(dir.to_string_lossy().to_string(), None::<&str>)
        .map_err(|e| e.to_string())
}
