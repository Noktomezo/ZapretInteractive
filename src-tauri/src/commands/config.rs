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
    #[serde(default)]
    pub system: bool,
    #[serde(
        default,
        rename = "systemBaseName",
        skip_serializing_if = "Option::is_none"
    )]
    pub system_base_name: Option<String>,
    #[serde(
        default,
        rename = "systemBaseContent",
        skip_serializing_if = "Option::is_none"
    )]
    pub system_base_content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: String,
    pub name: String,
    pub strategies: Vec<Strategy>,
    #[serde(default)]
    pub system: bool,
    #[serde(
        default,
        rename = "systemBaseName",
        skip_serializing_if = "Option::is_none"
    )]
    pub system_base_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Placeholder {
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub system: bool,
    #[serde(
        default,
        rename = "systemBaseName",
        skip_serializing_if = "Option::is_none"
    )]
    pub system_base_name: Option<String>,
    #[serde(
        default,
        rename = "systemBasePath",
        skip_serializing_if = "Option::is_none"
    )]
    pub system_base_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Filter {
    pub id: String,
    pub name: String,
    pub filename: String,
    pub active: bool,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub system: bool,
    #[serde(
        default,
        rename = "systemBaseName",
        skip_serializing_if = "Option::is_none"
    )]
    pub system_base_name: Option<String>,
    #[serde(
        default,
        rename = "systemBaseFilename",
        skip_serializing_if = "Option::is_none"
    )]
    pub system_base_filename: Option<String>,
    #[serde(
        default,
        rename = "systemBaseContent",
        skip_serializing_if = "Option::is_none"
    )]
    pub system_base_content: Option<String>,
    #[serde(
        default,
        rename = "systemBaseActive",
        skip_serializing_if = "Option::is_none"
    )]
    pub system_base_active: Option<bool>,
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
    #[serde(default, rename = "systemRemovedCategoryIds")]
    pub system_removed_category_ids: Vec<String>,
    #[serde(default, rename = "systemRemovedStrategyKeys")]
    pub system_removed_strategy_keys: Vec<String>,
    #[serde(default, rename = "systemRemovedPlaceholderNames")]
    pub system_removed_placeholder_names: Vec<String>,
    #[serde(default, rename = "systemRemovedFilterIds")]
    pub system_removed_filter_ids: Vec<String>,
    #[serde(default, rename = "systemSyncInitialized")]
    pub system_sync_initialized: bool,
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

fn system_strategy_key(category_id: &str, strategy_id: &str) -> String {
    format!("{category_id}::{strategy_id}")
}

fn annotate_builtin_category(category: &mut Category) {
    category.system = true;
    category.system_base_name = Some(category.name.clone());

    for strategy in &mut category.strategies {
        strategy.system = true;
        strategy.system_base_name = Some(strategy.name.clone());
        strategy.system_base_content = Some(strategy.content.clone());
    }
}

fn annotate_builtin_placeholder(placeholder: &mut Placeholder) {
    placeholder.system = true;
    placeholder.system_base_name = Some(placeholder.name.clone());
    placeholder.system_base_path = Some(placeholder.path.clone());
}

fn annotate_builtin_filter(filter: &mut Filter) {
    filter.system = true;
    filter.system_base_name = Some(filter.name.clone());
    filter.system_base_filename = Some(filter.filename.clone());
    filter.system_base_content = Some(filter.content.clone());
    filter.system_base_active = Some(filter.active);
}

fn strategy_base_name(strategy: &Strategy) -> &str {
    strategy
        .system_base_name
        .as_deref()
        .unwrap_or(strategy.name.as_str())
}

fn strategy_base_content(strategy: &Strategy) -> &str {
    strategy
        .system_base_content
        .as_deref()
        .unwrap_or(strategy.content.as_str())
}

fn is_system_strategy_modified(strategy: &Strategy) -> bool {
    strategy.name != strategy_base_name(strategy)
        || strategy.content != strategy_base_content(strategy)
}

fn category_base_name(category: &Category) -> &str {
    category
        .system_base_name
        .as_deref()
        .unwrap_or(category.name.as_str())
}

fn is_system_category_name_modified(category: &Category) -> bool {
    category.name != category_base_name(category)
}

fn is_system_category_modified(category: &Category) -> bool {
    if is_system_category_name_modified(category) {
        return true;
    }

    category
        .strategies
        .iter()
        .any(|strategy| !strategy.system || is_system_strategy_modified(strategy))
}

fn placeholder_base_name(placeholder: &Placeholder) -> &str {
    placeholder
        .system_base_name
        .as_deref()
        .unwrap_or(placeholder.name.as_str())
}

fn placeholder_base_path(placeholder: &Placeholder) -> &str {
    placeholder
        .system_base_path
        .as_deref()
        .unwrap_or(placeholder.path.as_str())
}

fn is_system_placeholder_modified(placeholder: &Placeholder) -> bool {
    placeholder.name != placeholder_base_name(placeholder)
        || placeholder.path != placeholder_base_path(placeholder)
}

fn filter_base_name(filter: &Filter) -> &str {
    filter
        .system_base_name
        .as_deref()
        .unwrap_or(filter.name.as_str())
}

fn filter_base_filename(filter: &Filter) -> &str {
    filter
        .system_base_filename
        .as_deref()
        .unwrap_or(filter.filename.as_str())
}

fn filter_base_content(filter: &Filter) -> &str {
    filter
        .system_base_content
        .as_deref()
        .unwrap_or(filter.content.as_str())
}

fn filter_base_active(filter: &Filter) -> bool {
    filter.system_base_active.unwrap_or(filter.active)
}

fn is_system_filter_modified(filter: &Filter) -> bool {
    filter.name != filter_base_name(filter)
        || filter.filename != filter_base_filename(filter)
        || filter.content != filter_base_content(filter)
        || filter.active != filter_base_active(filter)
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
            system: false,
            system_base_name: None,
            system_base_filename: None,
            system_base_content: None,
            system_base_active: None,
        },
        Filter {
            id: "filter-stun".to_string(),
            name: "STUN".to_string(),
            filename: "windivert_part.stun.txt".to_string(),
            active: true,
            content: String::new(),
            system: false,
            system_base_name: None,
            system_base_filename: None,
            system_base_content: None,
            system_base_active: None,
        },
        Filter {
            id: "filter-wireguard".to_string(),
            name: "WireGuard".to_string(),
            filename: "windivert_part.wireguard.txt".to_string(),
            active: false,
            content: String::new(),
            system: false,
            system_base_name: None,
            system_base_filename: None,
            system_base_content: None,
            system_base_active: None,
        },
        Filter {
            id: "filter-quic".to_string(),
            name: "QUIC Initial IETF".to_string(),
            filename: "windivert_part.quic_initial_ietf.txt".to_string(),
            active: false,
            content: String::new(),
            system: false,
            system_base_name: None,
            system_base_filename: None,
            system_base_content: None,
            system_base_active: None,
        },
        Filter {
            id: "filter-dht".to_string(),
            name: "DHT".to_string(),
            filename: "windivert_part.dht.txt".to_string(),
            active: false,
            content: String::new(),
            system: false,
            system_base_name: None,
            system_base_filename: None,
            system_base_content: None,
            system_base_active: None,
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
        config.system_removed_category_ids.clear();
        config.system_removed_strategy_keys.clear();
        config.system_removed_placeholder_names.clear();
        config.system_removed_filter_ids.clear();
        config.system_sync_initialized = true;
        if config.filters.is_empty() {
            config.filters = default_filters_metadata();
        }
        populate_builtin_filter_content(&mut config.filters);
        normalize_placeholder_paths(&mut config.placeholders);
        for placeholder in &mut config.placeholders {
            annotate_builtin_placeholder(placeholder);
        }
        for filter in &mut config.filters {
            annotate_builtin_filter(filter);
        }
        for category in &mut config.categories {
            annotate_builtin_category(category);
        }
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
    let builtin_config = AppConfig::default();

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

    if sync_builtin_placeholders(&mut config, &builtin_config) {
        changed = true;
    }

    if sync_builtin_filters(&mut config, &builtin_config) {
        changed = true;
    }

    if sync_builtin_categories(&mut config, &builtin_config) {
        changed = true;
    }

    for filter in config.filters.iter_mut() {
        if !filter.content.is_empty() {
            let filter_path = get_runtime_data_dir()
                .join("filters")
                .join(&filter.filename);
            if let Ok(existing_content) = fs::read_to_string(&filter_path) {
                if existing_content != filter.content {
                    let _ = fs::write(&filter_path, &filter.content);
                }
            } else if !filter.content.is_empty() {
                if let Some(parent) = filter_path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                let _ = fs::write(&filter_path, &filter.content);
            }
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

fn sync_builtin_placeholders(config: &mut AppConfig, builtin_config: &AppConfig) -> bool {
    let mut changed = false;
    let allow_additions = config.system_sync_initialized;

    config.system_removed_placeholder_names.sort();
    config.system_removed_placeholder_names.dedup();

    for placeholder in &mut config.placeholders {
        if let Some(builtin_placeholder) = builtin_config.placeholders.iter().find(|item| {
            item.name == placeholder.name
                || placeholder
                    .system_base_name
                    .as_deref()
                    .is_some_and(|base_name| item.name == base_name)
        }) && sync_builtin_placeholder(placeholder, builtin_placeholder)
        {
            changed = true;
        }
    }

    let before_count = config.placeholders.len();
    config.placeholders.retain(|placeholder| {
        if builtin_config
            .placeholders
            .iter()
            .any(|builtin| builtin.name == placeholder.name)
        {
            return true;
        }

        !placeholder.system || is_system_placeholder_modified(placeholder)
    });
    if config.placeholders.len() != before_count {
        changed = true;
    }

    for builtin_placeholder in &builtin_config.placeholders {
        let already_present = config
            .placeholders
            .iter()
            .any(|placeholder| placeholder.name == builtin_placeholder.name);
        let removed_by_user = config
            .system_removed_placeholder_names
            .iter()
            .any(|name| name == &builtin_placeholder.name);

        if allow_additions && !already_present && !removed_by_user {
            config.placeholders.push(builtin_placeholder.clone());
            changed = true;
        }
    }

    changed
}

fn sync_builtin_placeholder(
    placeholder: &mut Placeholder,
    builtin_placeholder: &Placeholder,
) -> bool {
    let mut changed = false;

    if !placeholder.system {
        placeholder.system = true;
        changed = true;
    }

    if placeholder.system_base_name.is_none() {
        placeholder.system_base_name = Some(builtin_placeholder.name.clone());
        changed = true;
    }

    if placeholder.system_base_path.is_none() {
        placeholder.system_base_path = Some(builtin_placeholder.path.clone());
        changed = true;
    }

    if !is_system_placeholder_modified(placeholder) {
        if placeholder.name != builtin_placeholder.name {
            placeholder.name = builtin_placeholder.name.clone();
            changed = true;
        }
        if placeholder.path != builtin_placeholder.path {
            placeholder.path = builtin_placeholder.path.clone();
            changed = true;
        }
        if placeholder.system_base_name.as_deref() != Some(builtin_placeholder.name.as_str()) {
            placeholder.system_base_name = Some(builtin_placeholder.name.clone());
            changed = true;
        }
        if placeholder.system_base_path.as_deref() != Some(builtin_placeholder.path.as_str()) {
            placeholder.system_base_path = Some(builtin_placeholder.path.clone());
            changed = true;
        }
    }

    changed
}

fn sync_builtin_filters(config: &mut AppConfig, builtin_config: &AppConfig) -> bool {
    let mut changed = false;
    let allow_additions = config.system_sync_initialized;

    config.system_removed_filter_ids.sort();
    config.system_removed_filter_ids.dedup();

    for filter in &mut config.filters {
        if let Some(builtin_filter) = builtin_config
            .filters
            .iter()
            .find(|item| item.id == filter.id)
            && sync_builtin_filter(filter, builtin_filter)
        {
            changed = true;
        }
    }

    let before_count = config.filters.len();
    config.filters.retain(|filter| {
        if builtin_config
            .filters
            .iter()
            .any(|builtin| builtin.id == filter.id)
        {
            return true;
        }

        !filter.system || is_system_filter_modified(filter)
    });
    if config.filters.len() != before_count {
        changed = true;
    }

    for builtin_filter in &builtin_config.filters {
        let already_present = config
            .filters
            .iter()
            .any(|filter| filter.id == builtin_filter.id);
        let removed_by_user = config
            .system_removed_filter_ids
            .iter()
            .any(|id| id == &builtin_filter.id);

        if allow_additions && !already_present && !removed_by_user {
            config.filters.push(builtin_filter.clone());
            changed = true;
        }
    }

    changed
}

fn sync_builtin_filter(filter: &mut Filter, builtin_filter: &Filter) -> bool {
    let mut changed = false;

    if !filter.system {
        filter.system = true;
        changed = true;
    }

    if filter.system_base_name.is_none() {
        filter.system_base_name = Some(builtin_filter.name.clone());
        changed = true;
    }

    if filter.system_base_filename.is_none() {
        filter.system_base_filename = Some(builtin_filter.filename.clone());
        changed = true;
    }

    if filter.system_base_content.is_none() {
        filter.system_base_content = Some(builtin_filter.content.clone());
        changed = true;
    }

    if filter.system_base_active.is_none() {
        filter.system_base_active = Some(builtin_filter.active);
        changed = true;
    }

    if !is_system_filter_modified(filter) {
        if filter.name != builtin_filter.name {
            filter.name = builtin_filter.name.clone();
            changed = true;
        }
        if filter.filename != builtin_filter.filename {
            filter.filename = builtin_filter.filename.clone();
            changed = true;
        }
        if filter.content != builtin_filter.content {
            filter.content = builtin_filter.content.clone();
            changed = true;
        }
        if filter.active != builtin_filter.active {
            filter.active = builtin_filter.active;
            changed = true;
        }
        if filter.system_base_name.as_deref() != Some(builtin_filter.name.as_str()) {
            filter.system_base_name = Some(builtin_filter.name.clone());
            changed = true;
        }
        if filter.system_base_filename.as_deref() != Some(builtin_filter.filename.as_str()) {
            filter.system_base_filename = Some(builtin_filter.filename.clone());
            changed = true;
        }
        if filter.system_base_content.as_deref() != Some(builtin_filter.content.as_str()) {
            filter.system_base_content = Some(builtin_filter.content.clone());
            changed = true;
        }
        if filter.system_base_active != Some(builtin_filter.active) {
            filter.system_base_active = Some(builtin_filter.active);
            changed = true;
        }
    }

    changed
}

fn sync_builtin_categories(config: &mut AppConfig, builtin_config: &AppConfig) -> bool {
    let mut changed = false;
    let allow_additions = config.system_sync_initialized;

    config.system_removed_category_ids.sort();
    config.system_removed_category_ids.dedup();
    config.system_removed_strategy_keys.sort();
    config.system_removed_strategy_keys.dedup();

    for category in &mut config.categories {
        if let Some(builtin_category) = builtin_config
            .categories
            .iter()
            .find(|item| item.id == category.id)
            && sync_builtin_category(
                category,
                builtin_category,
                &config.system_removed_strategy_keys,
                allow_additions,
            )
        {
            changed = true;
        }
    }

    let before_category_count = config.categories.len();
    config.categories.retain(|category| {
        if builtin_config
            .categories
            .iter()
            .any(|builtin| builtin.id == category.id)
        {
            return true;
        }

        !category.system || is_system_category_modified(category)
    });
    if config.categories.len() != before_category_count {
        changed = true;
    }

    for builtin_category in &builtin_config.categories {
        let already_present = config
            .categories
            .iter()
            .any(|category| category.id == builtin_category.id);
        let removed_by_user = config
            .system_removed_category_ids
            .iter()
            .any(|id| id == &builtin_category.id);

        if allow_additions && !already_present && !removed_by_user {
            config.categories.push(builtin_category.clone());
            changed = true;
        }
    }

    if !config.system_sync_initialized {
        config.system_sync_initialized = true;
        changed = true;
    }

    changed
}

fn sync_builtin_category(
    category: &mut Category,
    builtin_category: &Category,
    removed_strategy_keys: &[String],
    allow_additions: bool,
) -> bool {
    let mut changed = false;

    if !category.system {
        category.system = true;
        changed = true;
    }

    if category.system_base_name.is_none() {
        category.system_base_name = Some(builtin_category.name.clone());
        changed = true;
    }

    if !is_system_category_name_modified(category) {
        if category.name != builtin_category.name {
            category.name = builtin_category.name.clone();
            changed = true;
        }
        if category.system_base_name.as_deref() != Some(builtin_category.name.as_str()) {
            category.system_base_name = Some(builtin_category.name.clone());
            changed = true;
        }
    }

    for strategy in &mut category.strategies {
        if let Some(builtin_strategy) = builtin_category
            .strategies
            .iter()
            .find(|item| item.id == strategy.id)
            && sync_builtin_strategy(strategy, builtin_strategy)
        {
            changed = true;
        }
    }

    let before_strategy_count = category.strategies.len();
    category.strategies.retain(|strategy| {
        if builtin_category
            .strategies
            .iter()
            .any(|builtin| builtin.id == strategy.id)
        {
            return true;
        }

        !strategy.system || is_system_strategy_modified(strategy)
    });
    if category.strategies.len() != before_strategy_count {
        changed = true;
    }

    for builtin_strategy in &builtin_category.strategies {
        let already_present = category
            .strategies
            .iter()
            .any(|strategy| strategy.id == builtin_strategy.id);
        let removed_by_user = removed_strategy_keys
            .iter()
            .any(|key| key == &system_strategy_key(&category.id, &builtin_strategy.id));

        if allow_additions && !already_present && !removed_by_user {
            category.strategies.push(builtin_strategy.clone());
            changed = true;
        }
    }

    changed
}

fn sync_builtin_strategy(strategy: &mut Strategy, builtin_strategy: &Strategy) -> bool {
    let mut changed = false;

    if !strategy.system {
        strategy.system = true;
        changed = true;
    }

    if strategy.system_base_name.is_none() {
        strategy.system_base_name = Some(builtin_strategy.name.clone());
        changed = true;
    }

    if strategy.system_base_content.is_none() {
        strategy.system_base_content = Some(builtin_strategy.content.clone());
        changed = true;
    }

    if !is_system_strategy_modified(strategy) {
        if strategy.name != builtin_strategy.name {
            strategy.name = builtin_strategy.name.clone();
            changed = true;
        }
        if strategy.content != builtin_strategy.content {
            strategy.content = builtin_strategy.content.clone();
            changed = true;
        }
        if strategy.system_base_name.as_deref() != Some(builtin_strategy.name.as_str()) {
            strategy.system_base_name = Some(builtin_strategy.name.clone());
            changed = true;
        }
        if strategy.system_base_content.as_deref() != Some(builtin_strategy.content.as_str()) {
            strategy.system_base_content = Some(builtin_strategy.content.clone());
            changed = true;
        }
    }

    changed
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
pub fn get_builtin_config() -> AppConfig {
    AppConfig::default()
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
