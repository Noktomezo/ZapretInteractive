use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
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
const SOURCE_MANAGED_DIR_NAME: &str = "thirdparty";
const INSTALLED_RESOURCES_DIR_NAME: &str = "resources";
const LEGACY_MANAGED_DIR_NAME: &str = ".zapret";
const LEGACY_MANAGED_PATH_ALIAS: &str = "@thirdparty";
const MANAGED_PATH_ALIAS: &str = "@resources";

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

#[derive(Debug, Clone, Copy, Serialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WindowMaterial {
    None,
    #[default]
    Acrylic,
    Mica,
    Tabbed,
}

impl<'de> Deserialize<'de> for WindowMaterial {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum WindowMaterialRepr {
            String(String),
            Bool(bool),
        }

        match WindowMaterialRepr::deserialize(deserializer)? {
            WindowMaterialRepr::String(value) => match value.as_str() {
                "none" => Ok(Self::None),
                "acrylic" => Ok(Self::Acrylic),
                "mica" => Ok(Self::Mica),
                "tabbed" => Ok(Self::Tabbed),
                other => Err(serde::de::Error::unknown_variant(
                    other,
                    &["none", "acrylic", "mica", "tabbed"],
                )),
            },
            WindowMaterialRepr::Bool(enabled) => {
                Ok(if enabled { Self::Acrylic } else { Self::None })
            }
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
        default = "default_window_material",
        rename = "windowMaterial",
        alias = "windowAcrylicEnabled"
    )]
    pub window_material: WindowMaterial,
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

fn default_window_material() -> WindowMaterial {
    WindowMaterial::Acrylic
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

fn is_dev_project_root(path: &Path) -> bool {
    path.join("src-tauri").join("tauri.conf.json").is_file()
        && path.join(SOURCE_MANAGED_DIR_NAME).is_dir()
}

fn executable_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
}

fn find_dev_project_root() -> Option<PathBuf> {
    if !cfg!(debug_assertions) {
        return None;
    }

    let mut candidates = Vec::new();

    if let Ok(path) = std::env::current_exe() {
        candidates.extend(path.ancestors().map(Path::to_path_buf));
    }

    if let Ok(path) = std::env::current_dir() {
        candidates.extend(path.ancestors().map(Path::to_path_buf));
    }

    candidates
        .into_iter()
        .find(|candidate| is_dev_project_root(candidate))
}

fn legacy_zapret_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(LEGACY_MANAGED_DIR_NAME)
}

fn install_resources_dir() -> PathBuf {
    executable_dir()
        .unwrap_or_else(|| {
            legacy_zapret_dir()
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from("."))
        })
        .join(INSTALLED_RESOURCES_DIR_NAME)
}

pub(crate) fn get_runtime_data_dir() -> PathBuf {
    executable_dir().unwrap_or_else(|| PathBuf::from("."))
}

fn managed_relative_path(path: &str) -> Option<String> {
    let normalized = path.replace('\\', "/");
    if normalized == MANAGED_PATH_ALIAS {
        return Some(String::new());
    }

    normalized
        .strip_prefix(&format!("{MANAGED_PATH_ALIAS}/"))
        .map(str::to_string)
}

fn join_relative_path(base: &Path, relative: &str) -> PathBuf {
    let mut path = base.to_path_buf();
    for segment in relative.split('/') {
        if !segment.is_empty() {
            path.push(segment);
        }
    }
    path
}

fn normalize_placeholder_path(path: &str) -> Option<String> {
    let normalized = path.replace('\\', "/");
    if normalized == MANAGED_PATH_ALIAS || normalized.starts_with(&format!("{MANAGED_PATH_ALIAS}/"))
    {
        return Some(normalized);
    }

    if normalized == LEGACY_MANAGED_PATH_ALIAS
        || normalized.starts_with(&format!("{LEGACY_MANAGED_PATH_ALIAS}/"))
    {
        return Some(normalized.replacen(LEGACY_MANAGED_PATH_ALIAS, MANAGED_PATH_ALIAS, 1));
    }

    let legacy_tilde = format!("~/{LEGACY_MANAGED_DIR_NAME}");
    if normalized == legacy_tilde {
        return Some(MANAGED_PATH_ALIAS.to_string());
    }
    if let Some(relative) = normalized.strip_prefix(&format!("{legacy_tilde}/")) {
        return Some(format!("{MANAGED_PATH_ALIAS}/{relative}"));
    }

    let legacy_absolute = legacy_zapret_dir().to_string_lossy().replace('\\', "/");
    if normalized == legacy_absolute {
        return Some(MANAGED_PATH_ALIAS.to_string());
    }
    normalized
        .strip_prefix(&format!("{legacy_absolute}/"))
        .map(|relative| format!("{MANAGED_PATH_ALIAS}/{relative}"))
}

fn normalize_placeholder_paths(placeholders: &mut [Placeholder]) -> bool {
    let mut changed = false;

    for placeholder in placeholders.iter_mut() {
        if let Some(normalized) = normalize_placeholder_path(&placeholder.path)
            && placeholder.path != normalized
        {
            placeholder.path = normalized;
            changed = true;
        }
    }

    changed
}

fn resolve_managed_placeholder_path(path: &str) -> Option<String> {
    let relative = managed_relative_path(path)?;
    let base = get_managed_resources_dir();
    Some(if relative.is_empty() {
        base.to_string_lossy().to_string()
    } else {
        join_relative_path(&base, &relative)
            .to_string_lossy()
            .to_string()
    })
}

fn copy_tree_if_missing(source: &Path, destination: &Path) -> Result<(), String> {
    if !source.exists() {
        return Ok(());
    }

    if source.is_file() {
        if destination.exists() {
            return Ok(());
        }

        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::copy(source, destination).map_err(|e| e.to_string())?;
        return Ok(());
    }

    fs::create_dir_all(destination).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(source).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        copy_tree_if_missing(&entry.path(), &destination.join(entry.file_name()))?;
    }
    Ok(())
}

fn migrate_legacy_managed_resources(target_dir: &Path) -> Result<(), String> {
    let legacy_dir = legacy_zapret_dir();
    if legacy_dir == target_dir || !legacy_dir.exists() {
        return Ok(());
    }

    copy_tree_if_missing(&legacy_dir, target_dir)
}

fn migrate_legacy_runtime_data(target_dir: &Path) -> Result<(), String> {
    let config_path = target_dir.join("config.json");
    if !config_path.exists() {
        for source in [
            get_managed_resources_dir().join("config.json"),
            legacy_zapret_dir().join("config.json"),
        ] {
            if source.exists() {
                copy_tree_if_missing(&source, &config_path)?;
                break;
            }
        }
    }

    let filters_dir = target_dir.join("filters");
    if !filters_dir.exists() {
        for source in [
            get_managed_resources_dir().join("filters"),
            legacy_zapret_dir().join("filters"),
        ] {
            if source.exists() {
                copy_tree_if_missing(&source, &filters_dir)?;
                break;
            }
        }
    }

    Ok(())
}

impl Default for AppConfig {
    fn default() -> Self {
        let mut config: AppConfig =
            serde_json::from_str(DEFAULT_CONFIG).expect("Failed to parse default config");
        config.binaries_path = get_managed_resources_dir().to_string_lossy().to_string();
        if config.filters.is_empty() {
            config.filters = default_filters_metadata();
        }
        populate_builtin_filter_content(&mut config.filters);
        normalize_placeholder_paths(&mut config.placeholders);
        config
    }
}

pub fn get_managed_resources_dir() -> PathBuf {
    if let Some(project_root) = find_dev_project_root() {
        return project_root.join(SOURCE_MANAGED_DIR_NAME);
    }

    install_resources_dir()
}

pub fn ensure_managed_resources_dir_ready() -> Result<PathBuf, String> {
    let dir = get_managed_resources_dir();
    if find_dev_project_root().is_none() {
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        migrate_legacy_managed_resources(&dir)?;
    }
    Ok(dir)
}

pub fn ensure_runtime_data_dir_ready() -> Result<PathBuf, String> {
    let dir = get_runtime_data_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    migrate_legacy_runtime_data(&dir)?;
    Ok(dir)
}

pub(crate) fn get_config_path() -> PathBuf {
    get_runtime_data_dir().join("config.json")
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
    ensure_runtime_data_dir_ready()?;
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
    let expected_binaries_path = get_managed_resources_dir().to_string_lossy().to_string();

    if config.binaries_path != expected_binaries_path {
        config.binaries_path = expected_binaries_path;
        changed = true;
    }

    if populate_builtin_filter_content(&mut config.filters) {
        changed = true;
    }

    if normalize_placeholder_paths(&mut config.placeholders) {
        changed = true;
    }

    for filter in config.filters.iter_mut() {
        if !filter.content.is_empty() {
            continue;
        }

        let filter_path = get_runtime_data_dir()
            .join("filters")
            .join(&filter.filename);
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
    let _ = ensure_runtime_data_dir_ready()?;
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
    let dir = ensure_runtime_data_dir_ready()?;
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
pub fn get_resources_directory() -> String {
    get_managed_resources_dir().to_string_lossy().to_string()
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
        let resolved_path = if let Some(path) = resolve_managed_placeholder_path(&placeholder.path)
        {
            path
        } else if placeholder.path.starts_with('~') {
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
