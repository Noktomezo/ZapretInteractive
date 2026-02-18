use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const DEFAULT_CONFIG: &str = include_str!("../../default-config.json");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalPorts {
    pub tcp: String,
    pub udp: String,
}

impl Default for GlobalPorts {
    fn default() -> Self {
        Self {
            tcp: "80,443".to_string(),
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
    #[serde(default = "default_list_mode", rename = "listMode")]
    pub list_mode: String,
}

fn default_minimize_to_tray() -> bool {
    true
}

fn default_list_mode() -> String {
    "ipset".to_string()
}

impl Default for AppConfig {
    fn default() -> Self {
        let mut config: AppConfig =
            serde_json::from_str(DEFAULT_CONFIG).expect("Failed to parse default config");
        config.binaries_path = get_zapret_dir().to_string_lossy().to_string();
        if config.filters.is_empty() {
            config.filters = vec![
                Filter {
                    id: "filter-discord".to_string(),
                    name: "Discord Media".to_string(),
                    filename: "windivert_part.discord_media.txt".to_string(),
                    active: true,
                },
                Filter {
                    id: "filter-stun".to_string(),
                    name: "STUN".to_string(),
                    filename: "windivert_part.stun.txt".to_string(),
                    active: true,
                },
                Filter {
                    id: "filter-wireguard".to_string(),
                    name: "WireGuard".to_string(),
                    filename: "windivert_part.wireguard.txt".to_string(),
                    active: false,
                },
                Filter {
                    id: "filter-quic".to_string(),
                    name: "QUIC Initial IETF".to_string(),
                    filename: "windivert_part.quic_initial_ietf.txt".to_string(),
                    active: false,
                },
                Filter {
                    id: "filter-dht".to_string(),
                    name: "DHT".to_string(),
                    filename: "windivert_part.dht.txt".to_string(),
                    active: false,
                },
            ];
        }
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

#[tauri::command]
pub fn ensure_config_dir() -> Result<String, String> {
    let dir = get_zapret_dir();
    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    Ok(dir.to_string_lossy().to_string())
}

#[tauri::command]
pub fn load_config() -> Result<AppConfig, String> {
    let config_path = get_config_path();

    if !config_path.exists() {
        let default_config = AppConfig::default();
        save_config(default_config.clone())?;
        return Ok(default_config);
    }

    let content = fs::read_to_string(&config_path).map_err(|e| e.to_string())?;
    let config: AppConfig = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    Ok(config)
}

#[tauri::command]
pub fn save_config(config: AppConfig) -> Result<(), String> {
    ensure_config_dir()?;
    let config_path = get_config_path();
    let content = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    fs::write(&config_path, content).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn reset_config() -> Result<AppConfig, String> {
    let default_config = AppConfig::default();
    save_config(default_config.clone())?;
    Ok(default_config)
}

#[tauri::command]
pub fn get_zapret_directory() -> String {
    get_zapret_dir().to_string_lossy().to_string()
}

#[tauri::command]
pub fn resolve_placeholders(content: String, placeholders: Vec<Placeholder>) -> String {
    let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));

    let mut result = content;

    for placeholder in placeholders {
        let regex = regex::Regex::new(&format!("\\{{\\{{{}}}\\}}", placeholder.name)).unwrap();
        let resolved_path = if placeholder.path.starts_with('~') {
            let relative = &placeholder.path[1..];
            let relative_trimmed = relative.trim_start_matches('/').trim_start_matches('\\');
            let mut path = home_dir.clone();
            for part in relative_trimmed.split(|c| c == '/' || c == '\\') {
                if !part.is_empty() {
                    path.push(part);
                }
            }
            path.to_string_lossy().to_string()
        } else {
            placeholder.path.clone()
        };
        result = regex.replace_all(&result, &resolved_path).to_string();
    }

    result
}
