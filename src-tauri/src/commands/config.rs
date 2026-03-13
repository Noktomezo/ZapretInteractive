use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

const DEFAULT_CONFIG: &str = include_str!("../../default-config.json");
const DEFAULT_FILTER_DHT: &str = include_str!("../../default-filters/windivert_part.dht.txt");
const DEFAULT_FILTER_DISCORD_MEDIA: &str =
    include_str!("../../default-filters/windivert_part.discord_media.txt");
const DEFAULT_FILTER_QUIC_INITIAL_IETF: &str =
    include_str!("../../default-filters/windivert_part.quic_initial_ietf.txt");
const DEFAULT_FILTER_STUN: &str = include_str!("../../default-filters/windivert_part.stun.txt");
const DEFAULT_FILTER_WIREGUARD: &str =
    include_str!("../../default-filters/windivert_part.wireguard.txt");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalPorts {
    pub tcp: String,
    pub udp: String,
}

impl Default for GlobalPorts {
    fn default() -> Self {
        Self {
            tcp: "1-65535".to_string(),
            udp: "1-65535".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Strategy {
    pub id: String,
    pub name: String,
    pub content: String,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: String,
    pub name: String,
    pub strategies: Vec<Strategy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Placeholder {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Filter {
    pub id: String,
    pub name: String,
    pub filename: String,
    pub active: bool,
    #[serde(default)]
    pub content: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ListMode {
    #[default]
    Ipset,
    Exclude,
}

impl std::fmt::Display for ListMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ListMode::Ipset => write!(f, "ipset"),
            ListMode::Exclude => write!(f, "exclude"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub global_ports: GlobalPorts,
    pub categories: Vec<Category>,
    pub placeholders: Vec<Placeholder>,
    #[serde(default)]
    pub filters: Vec<Filter>,
    pub binaries_path: String,
    #[serde(default = "default_minimize_to_tray", rename = "minimizeToTray")]
    pub minimize_to_tray: bool,
    #[serde(default = "default_launch_to_tray", rename = "launchToTray")]
    pub launch_to_tray: bool,
    #[serde(
        default = "default_connect_on_autostart",
        rename = "connectOnAutostart"
    )]
    pub connect_on_autostart: bool,
    #[serde(default, rename = "listMode")]
    pub list_mode: ListMode,
    #[serde(
        default = "default_core_file_update_prompts_enabled",
        rename = "coreFileUpdatePromptsEnabled"
    )]
    pub core_file_update_prompts_enabled: bool,
    #[serde(
        default = "default_app_auto_updates_enabled",
        rename = "appAutoUpdatesEnabled"
    )]
    pub app_auto_updates_enabled: bool,
    #[serde(
        default = "default_window_acrylic_enabled",
        rename = "windowAcrylicEnabled"
    )]
    pub window_acrylic_enabled: bool,
}

pub struct ConfigEnsureResult {
    pub config: AppConfig,
    pub restored_default: bool,
    pub normalized_and_persisted: bool,
    pub unrecoverable_filters: Vec<String>,
}

struct NormalizedConfigResult {
    config: AppConfig,
    changed: bool,
    unrecoverable_filters: Vec<String>,
}

fn default_minimize_to_tray() -> bool {
    true
}

fn default_launch_to_tray() -> bool {
    false
}

fn default_connect_on_autostart() -> bool {
    false
}

fn default_core_file_update_prompts_enabled() -> bool {
    true
}

fn default_app_auto_updates_enabled() -> bool {
    true
}

fn default_window_acrylic_enabled() -> bool {
    true
}

fn built_in_filter_content(filename: &str) -> Option<&'static str> {
    match filename.trim() {
        "windivert_part.dht.txt" => Some(DEFAULT_FILTER_DHT),
        "windivert_part.discord_media.txt" => Some(DEFAULT_FILTER_DISCORD_MEDIA),
        "windivert_part.quic_initial_ietf.txt" => Some(DEFAULT_FILTER_QUIC_INITIAL_IETF),
        "windivert_part.stun.txt" => Some(DEFAULT_FILTER_STUN),
        "windivert_part.wireguard.txt" => Some(DEFAULT_FILTER_WIREGUARD),
        _ => None,
    }
}

fn default_filters_metadata() -> Vec<Filter> {
    vec![
        Filter {
            id: "filter-discord".to_string(),
            name: "Discord Media".to_string(),
            filename: "windivert_part.discord_media.txt".to_string(),
            active: true,
            content: String::new(),
        },
        Filter {
            id: "filter-stun".to_string(),
            name: "STUN".to_string(),
            filename: "windivert_part.stun.txt".to_string(),
            active: true,
            content: String::new(),
        },
        Filter {
            id: "filter-wireguard".to_string(),
            name: "WireGuard".to_string(),
            filename: "windivert_part.wireguard.txt".to_string(),
            active: false,
            content: String::new(),
        },
        Filter {
            id: "filter-quic".to_string(),
            name: "QUIC Initial IETF".to_string(),
            filename: "windivert_part.quic_initial_ietf.txt".to_string(),
            active: false,
            content: String::new(),
        },
        Filter {
            id: "filter-dht".to_string(),
            name: "DHT".to_string(),
            filename: "windivert_part.dht.txt".to_string(),
            active: false,
            content: String::new(),
        },
    ]
}

fn populate_builtin_filter_content(filters: &mut [Filter]) -> bool {
    let mut changed = false;
    for filter in filters.iter_mut() {
        if filter.content.is_empty()
            && let Some(content) = built_in_filter_content(&filter.filename)
        {
            filter.content = content.to_string();
            changed = true;
        }
    }
    changed
}

impl Default for AppConfig {
    fn default() -> Self {
        let mut config: AppConfig =
            serde_json::from_str(DEFAULT_CONFIG).expect("Failed to parse default config");
        config.binaries_path = get_zapret_dir().to_string_lossy().to_string();
        if config.filters.is_empty() {
            config.filters = default_filters_metadata();
        }
        populate_builtin_filter_content(&mut config.filters);
        config
    }
}

pub fn get_zapret_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".zapret")
}

pub(crate) fn get_config_path() -> PathBuf {
    get_zapret_dir().join("config.json")
}

pub struct AppState {
    pub config: Mutex<AppConfig>,
}

impl AppState {
    pub fn new() -> Result<Self, String> {
        let ensured = ensure_config_exists_and_normalized()?;
        Ok(Self {
            config: Mutex::new(ensured.config),
        })
    }
}

fn read_config_from_disk() -> Result<Option<AppConfig>, String> {
    let config_path = get_config_path();

    if !config_path.try_exists().map_err(|e| e.to_string())? {
        return Ok(None);
    }

    if !config_path.is_file() {
        return Ok(None);
    }

    let content = fs::read_to_string(&config_path).map_err(|e| e.to_string())?;
    let config: AppConfig = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    Ok(Some(config))
}

fn normalize_config(mut config: AppConfig) -> NormalizedConfigResult {
    let mut changed = false;
    let mut unrecoverable_filters = Vec::new();
    let expected_binaries_path = get_zapret_dir().to_string_lossy().to_string();

    if config.binaries_path != expected_binaries_path {
        config.binaries_path = expected_binaries_path;
        changed = true;
    }

    if populate_builtin_filter_content(&mut config.filters) {
        changed = true;
    }

    for filter in config.filters.iter_mut() {
        if !filter.content.is_empty() {
            continue;
        }

        let filter_path = get_zapret_dir().join("filters").join(&filter.filename);
        match fs::read_to_string(&filter_path) {
            Ok(content) => {
                filter.content = content;
                changed = true;
            }
            Err(_) => {
                unrecoverable_filters.push(filter.filename.clone());
            }
        }
    }

    NormalizedConfigResult {
        config,
        changed,
        unrecoverable_filters,
    }
}

fn ensure_config_exists_and_normalized() -> Result<ConfigEnsureResult, String> {
    match read_config_from_disk()? {
        Some(config) => {
            let normalized = normalize_config(config);
            if normalized.changed {
                save_config_to_disk(&normalized.config)?;
            }
            Ok(ConfigEnsureResult {
                config: normalized.config,
                restored_default: false,
                normalized_and_persisted: normalized.changed,
                unrecoverable_filters: normalized.unrecoverable_filters,
            })
        }
        None => {
            let config = AppConfig::default();
            save_config_to_disk(&config)?;
            Ok(ConfigEnsureResult {
                config,
                restored_default: true,
                normalized_and_persisted: false,
                unrecoverable_filters: Vec::new(),
            })
        }
    }
}

pub fn ensure_config_exists_and_loaded(state: &AppState) -> Result<ConfigEnsureResult, String> {
    let ensured = ensure_config_exists_and_normalized()?;
    let mut cfg = state.config.lock().map_err(|e| e.to_string())?;
    *cfg = ensured.config.clone();
    drop(cfg);
    Ok(ensured)
}

pub fn current_config(state: &AppState) -> Result<AppConfig, String> {
    state
        .config
        .lock()
        .map(|cfg| cfg.clone())
        .map_err(|e| e.to_string())
}

pub fn save_config_to_disk(config: &AppConfig) -> Result<(), String> {
    let dir = get_zapret_dir();
    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    let config_path = get_config_path();
    let content = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    let temp_path = config_path.with_extension("json.tmp");
    let mut temp_file = fs::File::create(&temp_path).map_err(|e| e.to_string())?;
    use std::io::Write;
    temp_file
        .write_all(content.as_bytes())
        .map_err(|e| e.to_string())?;
    temp_file.sync_all().map_err(|e| e.to_string())?;
    drop(temp_file);
    fs::rename(&temp_path, &config_path).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn ensure_config_dir() -> Result<String, String> {
    let dir = get_zapret_dir();
    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    Ok(dir.to_string_lossy().to_string())
}

#[tauri::command]
pub fn load_config(state: tauri::State<'_, AppState>) -> Result<AppConfig, String> {
    ensure_config_exists_and_loaded(&state).map(|result| result.config)
}

#[tauri::command]
pub fn save_config(config: AppConfig, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let normalized = normalize_config(config);
    save_config_to_disk(&normalized.config)?;
    let mut cfg = state.config.lock().map_err(|e| e.to_string())?;
    *cfg = normalized.config;
    Ok(())
}

#[tauri::command]
pub fn reset_config(state: tauri::State<'_, AppState>) -> Result<AppConfig, String> {
    let default_config = AppConfig::default();
    save_config_to_disk(&default_config)?;
    let mut cfg = state.config.lock().map_err(|e| e.to_string())?;
    *cfg = default_config.clone();
    Ok(default_config)
}

#[tauri::command]
pub fn update_list_mode(
    app: tauri::AppHandle,
    mode: ListMode,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let next_config = {
        let cfg = state.config.lock().map_err(|e| e.to_string())?;
        if cfg.list_mode == mode {
            drop(cfg);
            crate::sync_list_mode_ui(&app, mode)?;
            return Ok(());
        }

        let mut next = cfg.clone();
        next.list_mode = mode;
        next
    };

    save_config_to_disk(&next_config)?;

    let mut cfg = state.config.lock().map_err(|e| e.to_string())?;
    *cfg = next_config;
    drop(cfg);
    crate::sync_list_mode_ui(&app, mode)?;
    Ok(())
}

#[tauri::command]
pub fn get_zapret_directory() -> String {
    get_zapret_dir().to_string_lossy().to_string()
}

#[tauri::command]
pub fn config_exists() -> bool {
    get_config_path().is_file()
}

#[tauri::command]
pub fn resolve_placeholders(content: String, placeholders: Vec<Placeholder>) -> String {
    let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));

    let mut result = content;

    for placeholder in placeholders {
        let resolved_path = if placeholder.path.starts_with('~') {
            let relative = &placeholder.path[1..];
            let relative_trimmed = relative.trim_start_matches('/').trim_start_matches('\\');
            let mut path = home_dir.clone();
            for part in relative_trimmed.split(['/', '\\']) {
                if !part.is_empty() {
                    path.push(part);
                }
            }
            path.to_string_lossy().to_string()
        } else {
            placeholder.path.clone()
        };

        let token = format!("{{{{{}}}}}", placeholder.name);
        result = result.replace(&token, &resolved_path);
    }

    result
}
