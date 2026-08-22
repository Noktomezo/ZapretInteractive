use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GlobalPorts {
    pub tcp: String,
    pub udp: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Strategy {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub category: String,
    #[serde(default, rename = "categoryId")]
    pub category_id: String,
    #[serde(default, rename = "categoryOrder")]
    pub category_order: Option<i32>,
    #[serde(default)]
    pub order: Option<i32>,
    #[serde(default)]
    pub description: Option<String>,
    pub content: String,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub system: bool,
    #[serde(default, rename = "systemBaseName")]
    pub system_base_name: Option<String>,
    #[serde(default, rename = "systemBaseContent")]
    pub system_base_content: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Category {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing)]
    pub strategies: Vec<Strategy>,
    #[serde(default)]
    pub system: bool,
    #[serde(default, rename = "systemBaseName")]
    pub system_base_name: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Placeholder {
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub system: bool,
    #[serde(default, rename = "systemBaseName")]
    pub system_base_name: Option<String>,
    #[serde(default, rename = "systemBasePath")]
    pub system_base_path: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Filter {
    pub id: String,
    pub name: String,
    pub filename: String,
    pub active: bool,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub system: bool,
    #[serde(default, rename = "systemBaseName")]
    pub system_base_name: Option<String>,
    #[serde(default, rename = "systemBaseFilename")]
    pub system_base_filename: Option<String>,
    #[serde(default, rename = "systemBaseContent")]
    pub system_base_content: Option<String>,
    #[serde(default, rename = "systemBaseActive")]
    pub system_base_active: Option<bool>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ListMode {
    #[default]
    Ipset,
    Exclude,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiscordActivity {
    #[default]
    Playing,
    Listening,
    Watching,
    Competing,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LanguagePreference {
    #[default]
    System,
    Ru,
    En,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemePreference {
    #[default]
    System,
    Dark,
    Light,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub global_ports: GlobalPorts,
    pub categories: Vec<Category>,
    pub placeholders: Vec<Placeholder>,
    #[serde(default)]
    pub filters: Vec<Filter>,
    pub binaries_path: String,
    #[serde(default)]
    pub theme: ThemePreference,
    #[serde(default)]
    pub language: LanguagePreference,
    #[serde(default = "default_true", rename = "acrylicMaterial")]
    pub acrylic_material: bool,
    #[serde(default = "default_dns_preset", rename = "dnsPresetId")]
    pub dns_preset_id: String,
    #[serde(default = "default_bootstrap", rename = "dnsBootstrapResolvers")]
    pub dns_bootstrap_resolvers: Vec<String>,
    #[serde(default, rename = "dnsAcceleratorEnabled")]
    pub dns_accelerator_enabled: bool,
    #[serde(default, rename = "dnsModuleEnabled")]
    pub dns_module_enabled: bool,
    #[serde(default = "default_proxy_port", rename = "tgWsProxyPort")]
    pub tg_ws_proxy_port: u16,
    #[serde(default = "default_tg_secret", rename = "tgWsProxySecret")]
    pub tg_ws_proxy_secret: String,
    #[serde(default, rename = "tgWsProxyModuleEnabled")]
    pub tg_ws_proxy_module_enabled: bool,
    #[serde(default, rename = "discordPresenceEnabled")]
    pub discord_presence_enabled: bool,
    #[serde(default, rename = "discordPresenceActivityType")]
    pub discord_presence_activity_type: DiscordActivity,
    #[serde(default = "default_true", rename = "minimizeToTray")]
    pub minimize_to_tray: bool,
    #[serde(default, rename = "launchToTray")]
    pub launch_to_tray: bool,
    #[serde(default, rename = "connectOnAutostart")]
    pub connect_on_autostart: bool,
    #[serde(default, rename = "sidebarCollapsed")]
    pub sidebar_collapsed: bool,
    #[serde(default)]
    pub list_mode: ListMode,
    #[serde(default = "default_true", rename = "coreFileUpdatePromptsEnabled")]
    pub core_file_update_prompts_enabled: bool,
    #[serde(default = "default_true", rename = "appAutoUpdatesEnabled")]
    pub app_auto_updates_enabled: bool,
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

fn default_dns_preset() -> String {
    "comss-one".to_string()
}

fn default_bootstrap() -> Vec<String> {
    vec!["77.88.8.8".into(), "1.1.1.1".into(), "8.8.8.8".into()]
}

const fn default_proxy_port() -> u16 {
    1443
}

const fn default_true() -> bool {
    true
}

fn default_tg_secret() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Disconnecting,
    Error,
}

impl ConnectionStatus {
    pub fn label(self) -> std::borrow::Cow<'static, str> {
        match self {
            Self::Disconnected => t!("home.status_disconnected"),
            Self::Connecting => t!("home.status_connecting"),
            Self::Connected => t!("home.status_connected"),
            Self::Disconnecting => t!("home.status_disconnecting"),
            Self::Error => t!("home.status_disconnected"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct LogEntry {
    pub timestamp: std::time::SystemTime,
    pub message: String,
}
