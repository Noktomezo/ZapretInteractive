use crate::commands::process::kill_windivert_service;
use crate::config::{
    AppConfig, AppState, current_config, ensure_config_exists_and_loaded,
    ensure_managed_resources_dir_ready, ensure_runtime_data_dir_ready, get_config_path,
    get_managed_resources_dir, get_runtime_data_dir, validate_filter_filename,
};
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
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;
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
    pub config_restored: bool,
    pub config_reloaded: bool,
    pub restored_files: Vec<String>,
    pub unrecoverable_filters: Vec<String>,
}

#[derive(Clone, serde::Serialize)]
pub struct AppHealthSnapshot {
    pub binaries_ok: bool,
    pub missing_critical_files: Vec<String>,
    pub available_updates: Vec<String>,
    pub available_updates_checked: bool,
    pub config_missing: bool,
}

#[derive(Clone, serde::Serialize, Default)]
pub struct EnsureManagedFilesResult {
    pub restored_files: Vec<String>,
    pub config_restored: bool,
    pub config_reloaded: bool,
    pub unrecoverable_filters: Vec<String>,
}

#[derive(Clone)]
struct TrackedFile {
    name: &'static str,
    group: &'static str,
    dest_path: PathBuf,
    url: String,
    required_for_health: bool,
    include_in_remote_updates: bool,
}

#[derive(Clone)]
struct ConfiguredFilterFile {
    filename: String,
    content: String,
    dest_path: PathBuf,
}

#[derive(Clone)]
struct LocalFileInspection {
    exists: bool,
    hash: Option<String>,
}

const THIRD_PARTY_BASE_URL: &str =
    "https://raw.githubusercontent.com/Noktomezo/ZapretInteractive/main/thirdparty";
const REMOTE_HASHES_URL: &str =
    "https://raw.githubusercontent.com/Noktomezo/ZapretInteractive/main/thirdparty/hashes.json";
const BINARIES: &[&str] = &["WinDivert.dll", "Monkey64.sys", "winws.exe", "cygwin1.dll"];
const FAKE_FILES_BASE_URL: &str =
    "https://raw.githubusercontent.com/Noktomezo/ZapretInteractive/main/thirdparty/fake";
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

const LISTS_BASE_URL: &str =
    "https://raw.githubusercontent.com/Noktomezo/ZapretInteractive/main/thirdparty/lists";
const LISTS: &[&str] = &[
    "zapret-hosts-google.txt",
    "zapret-hosts-user-exclude.txt",
    "zapret-ip-user.txt",
];

const FILTERS: &[&str] = &[
    "windivert_part.dht.txt",
    "windivert_part.discord_media.txt",
    "windivert_part.quic_initial_ietf.txt",
    "windivert_part.stun.txt",
    "windivert_part.wireguard.txt",
];

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

#[derive(Clone, Copy, PartialEq, Eq)]
enum DownloadMode {
    RepairManaged,
    RepairOrUpdate,
    ReinstallAll,
    ApplyCoreUpdates,
}

fn get_fake_dir() -> PathBuf {
    get_managed_resources_dir().join("fake")
}
fn get_lists_dir() -> PathBuf {
    get_managed_resources_dir().join("lists")
}
fn get_filters_dir() -> PathBuf {
    get_runtime_data_dir().join("filters")
}
fn get_hashes_path() -> PathBuf {
    get_managed_resources_dir().join("hashes.json")
}

fn sanitize_filename(filename: &str) -> Result<String, String> {
    let name = validate_filter_filename(filename)?;

    let file_stem = Path::new(&name)
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
        let path = get_managed_resources_dir().join(name);
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

    for name in LISTS {
        let path = get_lists_dir().join(name);
        if path.exists() {
            hashes.insert(hash_key("lists", name), calculate_sha256(&path)?);
        }
    }

    save_stored_hashes(&hashes)
}

fn create_http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent("Mozilla/5.0")
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())
}

async fn fetch_remote_hashes(client: &reqwest::Client) -> Result<HashMap<String, String>, String> {
    let bytes = download_bytes(client, REMOTE_HASHES_URL, "hashes.json").await?;
    serde_json::from_slice(&bytes).map_err(|e| format!("Failed to parse remote hashes.json: {e}"))
}

fn remote_hash_for<'a>(
    remote_hashes: &'a HashMap<String, String>,
    file: &TrackedFile,
) -> Result<&'a str, String> {
    remote_hashes
        .get(&tracked_key(file))
        .map(String::as_str)
        .ok_or_else(|| format!("Missing remote hash for {} ({})", file.name, file.group))
}

fn hash_key(group: &str, name: &str) -> String {
    format!("{group}:{name}")
}

fn thirdparty_url(relative_path: &str) -> String {
    format!("{THIRD_PARTY_BASE_URL}/{relative_path}")
}

fn tracked_file_url(group: &str, name: &str) -> String {
    match group {
        "binaries" => thirdparty_url(name),
        "fake" => format!("{FAKE_FILES_BASE_URL}/{name}"),
        "lists" => format!("{LISTS_BASE_URL}/{name}"),
        _ => unreachable!("unsupported tracked file group: {group}"),
    }
}

fn tracked_files() -> Vec<TrackedFile> {
    let mut files = Vec::new();

    for name in BINARIES {
        files.push(TrackedFile {
            name,
            group: "binaries",
            dest_path: get_managed_resources_dir().join(name),
            url: tracked_file_url("binaries", name),
            required_for_health: true,
            include_in_remote_updates: true,
        });
    }

    for name in FAKE_FILES {
        files.push(TrackedFile {
            name,
            group: "fake",
            dest_path: get_fake_dir().join(name),
            url: tracked_file_url("fake", name),
            required_for_health: true,
            include_in_remote_updates: true,
        });
    }

    for name in LISTS {
        files.push(TrackedFile {
            name,
            group: "lists",
            dest_path: get_lists_dir().join(name),
            url: tracked_file_url("lists", name),
            required_for_health: true,
            include_in_remote_updates: false,
        });
    }

    files
}

fn tracked_key(file: &TrackedFile) -> String {
    hash_key(file.group, file.name)
}

fn configured_filter_files(config: &AppConfig) -> Vec<ConfiguredFilterFile> {
    config
        .filters
        .iter()
        .map(|filter| ConfiguredFilterFile {
            filename: filter.filename.clone(),
            content: filter.content.clone(),
            dest_path: get_filters_dir().join(&filter.filename),
        })
        .collect()
}

fn configured_filter_display_name(filename: &str) -> String {
    format!("filters/{filename}")
}

fn configured_filter_is_healthy(file: &ConfiguredFilterFile) -> Result<bool, String> {
    if !file.dest_path.exists() {
        return Ok(false);
    }
    let actual = calculate_sha256(&file.dest_path)?;
    let expected = calculate_sha256_bytes(file.content.as_bytes());
    Ok(actual == expected)
}

fn inspect_local_files(
    files: &[TrackedFile],
) -> Result<HashMap<String, LocalFileInspection>, String> {
    let mut inspections = HashMap::new();

    for file in files {
        let inspection = if file.dest_path.exists() {
            LocalFileInspection {
                exists: true,
                hash: Some(calculate_sha256(&file.dest_path)?),
            }
        } else {
            LocalFileInspection {
                exists: false,
                hash: None,
            }
        };
        inspections.insert(tracked_key(file), inspection);
    }

    Ok(inspections)
}

async fn inspect_local_files_async(
    files: &[TrackedFile],
) -> Result<HashMap<String, LocalFileInspection>, String> {
    let owned_files = files.to_vec();
    tauri::async_runtime::spawn_blocking(move || inspect_local_files(&owned_files))
        .await
        .map_err(|e| format!("Failed to join local file inspection task: {e}"))?
}

fn tracked_file_is_healthy(
    file: &TrackedFile,
    stored_hashes: &HashMap<String, String>,
    inspections: &HashMap<String, LocalFileInspection>,
) -> bool {
    let Some(inspection) = inspections.get(&tracked_key(file)) else {
        return false;
    };
    if !inspection.exists {
        return false;
    }

    let Some(expected) = expected_hash(stored_hashes, file.group, file.name) else {
        return false;
    };

    inspection.hash.as_deref() == Some(expected.as_str())
}

fn compute_health_snapshot_fields(
    files: &[TrackedFile],
    stored_hashes: &HashMap<String, String>,
    inspections: &HashMap<String, LocalFileInspection>,
) -> (bool, Vec<String>) {
    let mut missing_critical_files = Vec::new();

    for file in files.iter().filter(|file| file.required_for_health) {
        if !tracked_file_is_healthy(file, stored_hashes, inspections) {
            missing_critical_files.push(file.name.to_string());
        }
    }

    (missing_critical_files.is_empty(), missing_critical_files)
}

fn backfill_missing_core_hashes(
    files: &[TrackedFile],
    stored_hashes: &HashMap<String, String>,
    inspections: &HashMap<String, LocalFileInspection>,
) -> Result<bool, String> {
    let mut additions = Vec::new();

    for file in files {
        let Some(inspection) = inspections.get(&tracked_key(file)) else {
            continue;
        };
        if !inspection.exists || inspection.hash.is_none() {
            continue;
        }
        if expected_hash(stored_hashes, file.group, file.name).is_some() {
            continue;
        }

        additions.push((
            hash_key(file.group, file.name),
            inspection.hash.clone().unwrap_or_default(),
        ));
    }

    if additions.is_empty() {
        return Ok(false);
    }

    update_hashes(|hashes| {
        for (key, value) in additions {
            hashes.insert(key, value);
        }
        Ok(())
    })?;

    Ok(true)
}

struct LocalHealthSnapshotFields {
    binaries_ok: bool,
    missing_critical_files: Vec<String>,
    config_missing: bool,
}

fn build_local_health_snapshot_with_inspections(
    state: &AppState,
    files: &[TrackedFile],
    inspections: &HashMap<String, LocalFileInspection>,
) -> Result<LocalHealthSnapshotFields, String> {
    ensure_helper_files()?;

    let mut stored_hashes = load_stored_hashes()?;
    if backfill_missing_core_hashes(files, &stored_hashes, inspections)? {
        stored_hashes = load_stored_hashes()?;
    }

    let (_, mut missing_critical_files) =
        compute_health_snapshot_fields(files, &stored_hashes, inspections);
    let config = current_config(state)?;

    for filter in configured_filter_files(&config) {
        if !configured_filter_is_healthy(&filter)? {
            missing_critical_files.push(configured_filter_display_name(&filter.filename));
        }
    }

    let config_missing = !get_config_path().exists();
    if config_missing {
        missing_critical_files.push("config.json".to_string());
    }

    missing_critical_files.sort();
    missing_critical_files.dedup();

    Ok(LocalHealthSnapshotFields {
        binaries_ok: missing_critical_files.is_empty(),
        missing_critical_files,
        config_missing,
    })
}

fn build_local_health_snapshot(state: &AppState) -> Result<LocalHealthSnapshotFields, String> {
    let files = tracked_files();
    let inspections = inspect_local_files(&files)?;
    build_local_health_snapshot_with_inspections(state, &files, &inspections)
}

async fn collect_available_updates_with_context(
    client: &reqwest::Client,
    files: &[TrackedFile],
    inspections: &HashMap<String, LocalFileInspection>,
) -> Result<Vec<String>, String> {
    let remote_hashes = Arc::new(fetch_remote_hashes(client).await?);
    let manual_update_files: Vec<_> = files
        .iter()
        .filter_map(|file| file.include_in_remote_updates.then_some(file.clone()))
        .collect();
    let results = stream::iter(manual_update_files.into_iter().map(|file| {
        let remote_hashes = remote_hashes.clone();
        let local_hash = inspections
            .get(&tracked_key(&file))
            .and_then(|value| value.hash.clone());
        async move {
            let remote_hash = remote_hash_for(&remote_hashes, &file)?;
            let changed = local_hash.as_deref() != Some(remote_hash);
            Ok::<Option<String>, String>(if changed {
                Some(file.name.to_string())
            } else {
                None
            })
        }
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

fn ensure_base_directories() -> Result<(), String> {
    let _ = ensure_managed_resources_dir_ready()?;
    let _ = ensure_runtime_data_dir_ready()?;
    for dir in [get_fake_dir(), get_lists_dir(), get_filters_dir()] {
        if !dir.exists() {
            fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn ensure_helper_files() -> Result<(), String> {
    ensure_base_directories()?;
    for legacy_path in [
        get_managed_resources_dir().join("lists-state.json"),
        get_runtime_data_dir().join("lists-state.json"),
    ] {
        if legacy_path.exists() {
            fs::remove_file(&legacy_path).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn binary_names() -> Vec<&'static str> {
    BINARIES.to_vec()
}

fn critical_files_ok(state: &AppState) -> Result<bool, String> {
    Ok(build_local_health_snapshot(state)?.binaries_ok)
}

fn collect_missing_critical_files(state: &AppState) -> Result<Vec<String>, String> {
    Ok(build_local_health_snapshot(state)?.missing_critical_files)
}

fn path_is_inside(path: &Path, dir: &Path) -> bool {
    path.starts_with(dir)
}

fn event_affects_lists(paths: &[PathBuf]) -> bool {
    let lists_dir = get_lists_dir();
    paths.iter().any(|path| path_is_inside(path, &lists_dir))
}

fn event_affects_tracked_files(paths: &[PathBuf]) -> bool {
    let base_dir = get_managed_resources_dir();
    let fake_dir = get_fake_dir();
    let filters_dir = get_filters_dir();
    let hashes_path = get_hashes_path();
    let config_path = get_config_path();
    let critical_binary_names = binary_names();
    paths.iter().any(|path| {
        path_is_inside(path, &fake_dir)
            || path_is_inside(path, &filters_dir)
            || path == &hashes_path
            || path == &config_path
            || critical_binary_names
                .iter()
                .any(|name| path == &base_dir.join(name))
            || LISTS.iter().any(|name| path == &get_lists_dir().join(name))
    })
}

pub fn start_files_watcher(app: AppHandle) -> Result<(), String> {
    ensure_base_directories()?;
    if FILES_WATCHER_STARTED.swap(true, Ordering::SeqCst) {
        return Ok(());
    }
    let managed_dir = get_managed_resources_dir();
    let runtime_dir = get_runtime_data_dir();

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

        if let Err(e) = watcher.watch(&managed_dir, RecursiveMode::Recursive) {
            FILES_WATCHER_STARTED.store(false, Ordering::SeqCst);
            let _ = app.emit(
                "files-health-watch-error",
                format!("Не удалось подписаться на каталог файлов: {e}"),
            );
            return;
        }
        if runtime_dir != managed_dir
            && let Err(e) = watcher.watch(&runtime_dir, RecursiveMode::Recursive)
        {
            FILES_WATCHER_STARTED.store(false, Ordering::SeqCst);
            let _ = app.emit(
                "files-health-watch-error",
                format!("Не удалось подписаться на каталог runtime-данных: {e}"),
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

            if !tracked_files_changed && !lists_changed {
                continue;
            }

            let state = app.state::<AppState>();
            let ensured =
                match tauri::async_runtime::block_on(ensure_managed_files_internal(&state)) {
                    Ok(result) => result,
                    Err(error) => {
                        let _ = app.emit(
                            "files-health-watch-error",
                            format!("Не удалось восстановить управляемые файлы: {error}"),
                        );
                        continue;
                    }
                };

            let payload = match build_local_health_snapshot(&state) {
                Ok(snapshot) => FileHealthChangedPayload {
                    binaries_ok: snapshot.binaries_ok,
                    lists_changed,
                    config_missing: snapshot.config_missing,
                    config_restored: ensured.config_restored,
                    config_reloaded: ensured.config_reloaded,
                    restored_files: ensured.restored_files.clone(),
                    unrecoverable_filters: ensured.unrecoverable_filters.clone(),
                },
                Err(error) => {
                    let _ = app.emit(
                        "files-health-watch-error",
                        format!("Не удалось получить локальный снимок файлов: {error}"),
                    );
                    continue;
                }
            };

            let _ = app.emit("files-health-changed", payload);
        }
    });

    Ok(())
}

async fn calculate_sha256_async(file_path: PathBuf) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || calculate_sha256(&file_path))
        .await
        .map_err(|e| format!("Failed to join SHA-256 task: {e}"))?
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

async fn write_bytes_atomic(dest_path: &Path, bytes: &[u8], name: &str) -> Result<(), String> {
    if let Some(parent) = dest_path.parent()
        && !parent.exists()
    {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("Failed to create parent directory for {name}: {e}"))?;
    }

    let temp_path = dest_path.with_extension("tmp");
    use tokio::io::AsyncWriteExt;
    let mut temp_file = tokio::fs::File::create(&temp_path)
        .await
        .map_err(|e| format!("Failed to create temp file for {name}: {e}"))?;
    temp_file
        .write_all(bytes)
        .await
        .map_err(|e| format!("Failed to write temp file for {name}: {e}"))?;
    temp_file
        .sync_all()
        .await
        .map_err(|e| format!("Failed to sync temp file for {name}: {e}"))?;
    tokio::fs::rename(&temp_path, dest_path)
        .await
        .map_err(|e| format!("Failed to rename temp file for {name}: {e}"))?;
    Ok(())
}

fn configs_equal(left: &AppConfig, right: &AppConfig) -> bool {
    serde_json::to_string(left).ok() == serde_json::to_string(right).ok()
}

async fn sync_configured_filter_files(
    config: &AppConfig,
    force_rewrite: bool,
) -> Result<Vec<String>, String> {
    let mut restored_files = Vec::new();

    for filter in configured_filter_files(config) {
        let healthy = configured_filter_is_healthy(&filter)?;
        if !force_rewrite && healthy {
            continue;
        }

        write_bytes_atomic(
            &filter.dest_path,
            filter.content.as_bytes(),
            &configured_filter_display_name(&filter.filename),
        )
        .await?;
        restored_files.push(configured_filter_display_name(&filter.filename));
    }

    Ok(restored_files)
}

async fn build_download_plan(
    mode: DownloadMode,
    client: &reqwest::Client,
    files: &[TrackedFile],
) -> Result<Vec<FileToDownload>, String> {
    let mut candidates: Vec<FileToDownload> = files
        .iter()
        .map(|file| FileToDownload {
            name: file.name.to_string(),
            url: file.url.clone(),
            dest_path: file.dest_path.clone(),
            hash_key: Some(hash_key(file.group, file.name)),
            phase: file.group.to_string(),
            cached_bytes: None,
        })
        .collect();

    if mode == DownloadMode::ReinstallAll {
        return Ok(candidates);
    }

    let mut stored_hashes = load_stored_hashes()?;
    let inspections = inspect_local_files_async(files).await?;
    if backfill_missing_core_hashes(files, &stored_hashes, &inspections)? {
        stored_hashes = load_stored_hashes()?;
    }

    let stored_hashes = Arc::new(stored_hashes);
    let inspections = Arc::new(inspections);
    let remote_hashes = match mode {
        DownloadMode::RepairOrUpdate | DownloadMode::ApplyCoreUpdates => {
            Some(Arc::new(fetch_remote_hashes(client).await?))
        }
        DownloadMode::RepairManaged | DownloadMode::ReinstallAll => None,
    };
    let results = stream::iter(candidates.drain(..).zip(files.iter().cloned()).map(
        |(file, tracked_file)| {
            let stored_hashes = stored_hashes.clone();
            let inspections = inspections.clone();
            let remote_hashes = remote_hashes.clone();
            async move {
                let local_inspection = inspections.get(&tracked_key(&tracked_file));
                let local_hash = local_inspection.and_then(|value| value.hash.as_deref());
                let is_healthy =
                    tracked_file_is_healthy(&tracked_file, &stored_hashes, &inspections);

                let checked = match mode {
                    DownloadMode::RepairManaged => {
                        if is_healthy {
                            (false, None)
                        } else {
                            (true, None)
                        }
                    }
                    DownloadMode::RepairOrUpdate => {
                        if !is_healthy {
                            (true, None)
                        } else if tracked_file.include_in_remote_updates {
                            let remote_hashes = remote_hashes
                                .as_ref()
                                .ok_or_else(|| "Remote hashes not loaded".to_string())?;
                            let remote_hash = remote_hash_for(remote_hashes, &tracked_file)?;
                            (local_hash != Some(remote_hash), None)
                        } else {
                            (false, None)
                        }
                    }
                    DownloadMode::ApplyCoreUpdates => {
                        if !tracked_file.include_in_remote_updates {
                            (false, None)
                        } else {
                            let remote_hashes = remote_hashes
                                .as_ref()
                                .ok_or_else(|| "Remote hashes not loaded".to_string())?;
                            let remote_hash = remote_hash_for(remote_hashes, &tracked_file)?;
                            (local_hash != Some(remote_hash), None)
                        }
                    }
                    DownloadMode::ReinstallAll => (true, None),
                };

                Ok::<Option<FileToDownload>, String>(if checked.0 || checked.1.is_some() {
                    Some(FileToDownload {
                        cached_bytes: checked.1,
                        ..file
                    })
                } else {
                    None
                })
            }
        },
    ))
    .buffer_unordered(FILES_CONCURRENCY_LIMIT)
    .collect::<Vec<_>>()
    .await;

    let mut filtered = Vec::new();
    for result in results {
        if let Some(file) = result? {
            filtered.push(file);
        }
    }

    Ok(filtered)
}

async fn execute_download_plan(
    app: Option<&AppHandle>,
    client: &reqwest::Client,
    files_to_download: &[FileToDownload],
) -> Result<Vec<String>, String> {
    let total_files = files_to_download.len();
    if let Some(app) = app {
        app.emit("download-start", total_files).ok();
    }

    if total_files == 0 {
        if let Some(app) = app {
            app.emit("download-complete", ()).ok();
        }
        return Ok(Vec::new());
    }

    #[cfg(windows)]
    {
        if let Err(e) = kill_windivert_service() {
            if e.contains("does not exist") || e.contains("marked for deletion") {
                eprintln!("Non-fatal WinDivert service state before download: {e}");
            } else {
                if let Some(app) = app {
                    app.emit(
                        "download-error",
                        format!("Failed to stop WinDivert service before download: {e}"),
                    )
                    .ok();
                }
                return Err(format!(
                    "Failed to stop WinDivert service before download: {e}"
                ));
            }
        }
        sleep(Duration::from_millis(300)).await;
    }

    let mut downloaded = Vec::new();
    for (current, file) in files_to_download.iter().enumerate() {
        if let Some(app) = app {
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
        }

        let bytes = if let Some(bytes) = &file.cached_bytes {
            bytes.clone()
        } else {
            download_bytes(client, &file.url, &file.name).await?
        };

        write_bytes_atomic(&file.dest_path, &bytes, &file.name).await?;

        if let Some(hash_key) = &file.hash_key {
            let hash = calculate_sha256_async(file.dest_path.clone()).await?;
            update_hashes(|hashes| {
                hashes.insert(hash_key.clone(), hash);
                Ok(())
            })?;
        }

        downloaded.push(match file.phase.as_str() {
            "fake" => format!("fake/{}", file.name),
            "lists" => format!("lists/{}", file.name),
            _ => file.name.clone(),
        });
    }

    if let Some(app) = app {
        app.emit("download-complete", ()).ok();
    }

    Ok(downloaded)
}

async fn ensure_managed_files_internal(
    state: &AppState,
) -> Result<EnsureManagedFilesResult, String> {
    ensure_base_directories()?;

    let previous_config = current_config(state)?;
    let ensured_config = ensure_config_exists_and_loaded(state)?;
    let config = ensured_config.config.clone();
    let config_reloaded = !ensured_config.restored_default
        && (ensured_config.normalized_and_persisted || !configs_equal(&previous_config, &config));

    let client = create_http_client()?;
    let core_downloads =
        build_download_plan(DownloadMode::RepairManaged, &client, &tracked_files()).await?;
    let mut restored_files = execute_download_plan(None, &client, &core_downloads).await?;
    restored_files.extend(sync_configured_filter_files(&config, false).await?);
    restored_files.sort();
    restored_files.dedup();

    Ok(EnsureManagedFilesResult {
        restored_files,
        config_restored: ensured_config.restored_default,
        config_reloaded,
        unrecoverable_filters: ensured_config.unrecoverable_filters,
    })
}

async fn refresh_lists_internal() -> Result<usize, String> {
    ensure_helper_files()?;

    let lists_dir = get_lists_dir();
    let client = create_http_client()?;
    let remote_hashes = fetch_remote_hashes(&client).await?;
    let mut updated_count = 0usize;
    for name in LISTS {
        let tracked_file = TrackedFile {
            name,
            group: "lists",
            dest_path: lists_dir.join(name),
            url: tracked_file_url("lists", name),
            required_for_health: true,
            include_in_remote_updates: false,
        };
        let remote_hash = remote_hash_for(&remote_hashes, &tracked_file)?.to_string();
        let file_path = lists_dir.join(name);
        let local_hash = if file_path.exists() {
            calculate_sha256(&file_path).ok()
        } else {
            None
        };

        if local_hash.as_deref() != Some(remote_hash.as_str()) {
            let bytes = download_bytes(&client, &tracked_file.url, name).await?;
            let temp_path = file_path.with_extension("tmp");
            use tokio::io::AsyncWriteExt;
            let mut temp_file = tokio::fs::File::create(&temp_path)
                .await
                .map_err(|e| format!("Failed to create temp file: {e}"))?;
            temp_file
                .write_all(&bytes)
                .await
                .map_err(|e| format!("Failed to write temp file: {e}"))?;
            temp_file
                .sync_all()
                .await
                .map_err(|e| format!("Failed to sync temp file: {e}"))?;
            tokio::fs::rename(&temp_path, &file_path)
                .await
                .map_err(|e| format!("Failed to rename temp file: {e}"))?;
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
    Ok(updated_count)
}

pub async fn restore_default_filters_internal() -> Result<(), String> {
    ensure_base_directories()?;
    let default_config = AppConfig::default();
    let built_in_filters: Vec<_> = default_config
        .filters
        .into_iter()
        .filter(|filter| FILTERS.contains(&filter.filename.as_str()))
        .collect();
    let config = AppConfig {
        filters: built_in_filters,
        ..AppConfig::default()
    };
    sync_configured_filter_files(&config, true).await?;
    Ok(())
}

async fn build_app_health_snapshot(
    force_remote_updates: bool,
    state: &AppState,
) -> Result<AppHealthSnapshot, String> {
    let files = tracked_files();
    let inspections = inspect_local_files_async(&files).await?;
    let local = build_local_health_snapshot_with_inspections(state, &files, &inspections)?;

    let (available_updates, available_updates_checked) = if force_remote_updates {
        match create_http_client() {
            Ok(client) => {
                match collect_available_updates_with_context(&client, &files, &inspections).await {
                    Ok(updates) => (updates, true),
                    Err(error) => {
                        eprintln!(
                            "Failed to collect remote available updates while building health snapshot: {error}"
                        );
                        (Vec::new(), false)
                    }
                }
            }
            Err(error) => {
                eprintln!("Failed to create HTTP client while building health snapshot: {error}");
                (Vec::new(), false)
            }
        }
    } else {
        (Vec::new(), false)
    };

    Ok(AppHealthSnapshot {
        binaries_ok: local.binaries_ok,
        missing_critical_files: local.missing_critical_files,
        available_updates,
        available_updates_checked,
        config_missing: local.config_missing,
    })
}

#[tauri::command]
pub fn verify_binaries(state: tauri::State<'_, AppState>) -> Result<bool, String> {
    critical_files_ok(&state)
}

#[tauri::command]
pub fn get_missing_critical_files(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<String>, String> {
    collect_missing_critical_files(&state)
}

#[tauri::command]
pub async fn get_app_health_snapshot(
    force_remote_updates: Option<bool>,
    state: tauri::State<'_, AppState>,
) -> Result<AppHealthSnapshot, String> {
    build_app_health_snapshot(force_remote_updates.unwrap_or(false), &state).await
}

#[tauri::command]
pub async fn ensure_managed_files(
    state: tauri::State<'_, AppState>,
) -> Result<EnsureManagedFilesResult, String> {
    ensure_managed_files_internal(&state).await
}

#[tauri::command]
pub async fn apply_core_file_updates(app: AppHandle) -> Result<(), String> {
    ensure_helper_files()?;
    let client = create_http_client()?;
    let files = tracked_files();
    let files_to_download =
        build_download_plan(DownloadMode::ApplyCoreUpdates, &client, &files).await?;
    let updated_files = execute_download_plan(Some(&app), &client, &files_to_download).await?;

    if updated_files.is_empty() {
        let _ = app
            .notification()
            .builder()
            .title("Готово")
            .body("Удалённых обновлений для winws/fake файлов не найдено")
            .show();
    } else {
        let _ = app
            .notification()
            .builder()
            .title("Готово")
            .body(format!(
                "Обновлено {} файлов приложения",
                updated_files.len()
            ))
            .show();
    }

    Ok(())
}

#[tauri::command]
pub fn restore_hashes_from_disk() -> Result<(), String> {
    rebuild_hashes_from_disk()
}

#[tauri::command]
pub async fn download_binaries(app: AppHandle, force_all: Option<bool>) -> Result<(), String> {
    ensure_helper_files()?;
    let force_all = force_all.unwrap_or(false);

    let client = create_http_client()?;
    let files = tracked_files();
    let files_to_download = if force_all {
        build_download_plan(DownloadMode::ReinstallAll, &client, &files).await?
    } else {
        build_download_plan(DownloadMode::RepairOrUpdate, &client, &files).await?
    };
    let mut updated_files = execute_download_plan(Some(&app), &client, &files_to_download).await?;
    let filter_config = AppConfig {
        filters: current_config(&app.state::<AppState>())?.filters,
        ..AppConfig::default()
    };
    updated_files.extend(sync_configured_filter_files(&filter_config, force_all).await?);
    updated_files.sort();
    updated_files.dedup();

    let _ = app
        .notification()
        .builder()
        .title("Готово")
        .body(if updated_files.is_empty() {
            "Все управляемые файлы уже актуальны".to_string()
        } else {
            format!("Обновлено {} файлов приложения", updated_files.len())
        })
        .show();
    Ok(())
}

#[tauri::command]
pub async fn refresh_lists_if_stale() -> Result<usize, String> {
    refresh_lists_internal().await
}
#[tauri::command]
pub async fn restore_default_filters() -> Result<(), String> {
    restore_default_filters_internal().await
}
#[tauri::command]
pub fn get_binary_path(filename: &str) -> Result<String, String> {
    let filename = sanitize_filename(filename)?;
    if !BINARIES.iter().any(|name| *name == filename) {
        return Err("Unknown binary filename".to_string());
    }
    Ok(get_managed_resources_dir()
        .join(filename)
        .to_string_lossy()
        .to_string())
}
#[tauri::command]
pub fn get_winws_path() -> String {
    get_managed_resources_dir()
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
    let temp_path = file_path.with_extension("tmp");
    use std::io::Write;
    let mut temp_file =
        fs::File::create(&temp_path).map_err(|e| format!("Failed to create temp file: {e}"))?;
    temp_file
        .write_all(content.as_bytes())
        .map_err(|e| format!("Failed to write temp file: {e}"))?;
    temp_file
        .sync_all()
        .map_err(|e| format!("Failed to sync temp file: {e}"))?;
    drop(temp_file);
    fs::rename(&temp_path, &file_path).map_err(|e| format!("Failed to rename temp file: {e}"))?;

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
    Ok(())
}

#[tauri::command]
pub fn open_app_directory(app: AppHandle) -> Result<(), String> {
    let dir = ensure_runtime_data_dir_ready()?;
    app.opener()
        .open_path(dir.to_string_lossy().to_string(), None::<&str>)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_filters_directory(app: AppHandle) -> Result<(), String> {
    let dir = get_filters_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    app.opener()
        .open_path(dir.to_string_lossy().to_string(), None::<&str>)
        .map_err(|e| e.to_string())
}
