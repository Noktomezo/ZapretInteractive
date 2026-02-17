use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Clone, serde::Serialize)]
struct DownloadProgress {
    current: usize,
    total: usize,
    filename: String,
    phase: String,
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

fn get_zapret_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".zapret")
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

fn calculate_sha256(file_path: &PathBuf) -> Result<String, String> {
    let mut hasher = Sha256::new();
    let mut file = fs::File::open(file_path).map_err(|e| e.to_string())?;
    std::io::copy(&mut file, &mut hasher).map_err(|e| e.to_string())?;
    Ok(format!("{:x}", hasher.finalize()))
}

fn load_stored_hashes() -> std::collections::HashMap<String, String> {
    let hashes_path = get_hashes_path();
    if !hashes_path.exists() {
        return std::collections::HashMap::new();
    }
    
    let content = fs::read_to_string(&hashes_path).unwrap_or_default();
    serde_json::from_str(&content).unwrap_or_default()
}

fn save_stored_hashes(hashes: &std::collections::HashMap<String, String>) -> Result<(), String> {
    let hashes_path = get_hashes_path();
    let content = serde_json::to_string_pretty(hashes).map_err(|e| e.to_string())?;
    fs::write(&hashes_path, content).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn verify_binaries() -> Result<bool, String> {
    let dir = get_zapret_dir();
    let fake_dir = get_fake_dir();
    let lists_dir = get_lists_dir();
    let filters_dir = get_filters_dir();
    let stored_hashes = load_stored_hashes();
    
    // Check main binaries
    if stored_hashes.is_empty() {
        for (name, _) in BINARIES.iter() {
            let file_path = dir.join(name);
            if !file_path.exists() {
                return Ok(false);
            }
        }
        // Check fake files
        for name in FAKE_FILES.iter() {
            let file_path = fake_dir.join(name);
            if !file_path.exists() {
                return Ok(false);
            }
        }
        // Check lists
        for name in LISTS.iter() {
            let file_path = lists_dir.join(name);
            if !file_path.exists() {
                return Ok(false);
            }
        }
        // Check filters
        for name in FILTERS.iter() {
            let file_path = filters_dir.join(name);
            if !file_path.exists() {
                return Ok(false);
            }
        }
        return Ok(true);
    }
    
    for (name, _) in BINARIES.iter() {
        let file_path = dir.join(name);
        if !file_path.exists() {
            return Ok(false);
        }
        
        let expected_hash = match stored_hashes.get(*name) {
            Some(h) => h,
            None => return Ok(true),
        };
        
        let actual_hash = calculate_sha256(&file_path)?;
        if actual_hash != *expected_hash {
            return Ok(false);
        }
    }
    
    // Check fake files existence
    for name in FAKE_FILES.iter() {
        let file_path = fake_dir.join(name);
        if !file_path.exists() {
            return Ok(false);
        }
    }
    
    // Check lists existence
    for name in LISTS.iter() {
        let file_path = lists_dir.join(name);
        if !file_path.exists() {
            return Ok(false);
        }
    }
    
    // Check filters existence
    for name in FILTERS.iter() {
        let file_path = filters_dir.join(name);
        if !file_path.exists() {
            return Ok(false);
        }
    }
    
    Ok(true)
}

#[tauri::command]
pub async fn download_binaries(app: AppHandle) -> Result<(), String> {
    // Stop WinDivert driver before downloading
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("sc")
            .args(["stop", "WinDivert"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
        std::thread::sleep(std::time::Duration::from_millis(500));
        let _ = std::process::Command::new("sc")
            .args(["delete", "WinDivert"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    
    let dir = get_zapret_dir();
    let fake_dir = get_fake_dir();
    let lists_dir = get_lists_dir();
    let filters_dir = get_filters_dir();
    
    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    
    if !fake_dir.exists() {
        fs::create_dir_all(&fake_dir).map_err(|e| e.to_string())?;
    }
    
    if !lists_dir.exists() {
        fs::create_dir_all(&lists_dir).map_err(|e| e.to_string())?;
    }
    
    if !filters_dir.exists() {
        fs::create_dir_all(&filters_dir).map_err(|e| e.to_string())?;
    }
    
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0")
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;
    
    let mut hashes: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let total_files = BINARIES.len() + FAKE_FILES.len() + LISTS.len() + FILTERS.len();
    let mut current = 0;
    
    // Emit download start
    app.emit("download-start", total_files).ok();
    
    // Download main binaries
    for (name, url) in BINARIES.iter() {
        current += 1;
        
        println!("[download] Starting {} from {}", name, url);
        
        app.emit("download-progress", DownloadProgress {
            current,
            total: total_files,
            filename: name.to_string(),
            phase: "binaries".to_string(),
        }).ok();
        
        let file_path = dir.join(name);
        
        let response = client.get(*url)
            .send()
            .await
            .map_err(|e| {
                let err = format!("Failed to fetch {}: {}", name, e);
                println!("[download] ERROR: {}", err);
                app.emit("download-error", err.clone()).ok();
                err
            })?;
        
        if !response.status().is_success() {
            let err = format!("Failed to download {}: HTTP {}", name, response.status());
            println!("[download] ERROR: {}", err);
            app.emit("download-error", err.clone()).ok();
            return Err(err);
        }
        
        let bytes = response.bytes().await.map_err(|e| {
            let err = format!("Failed to read {} body: {}", name, e);
            println!("[download] ERROR: {}", err);
            err
        })?;
        
        fs::write(&file_path, &bytes).map_err(|e| {
            let err = format!("Failed to write {}: {}", name, e);
            println!("[download] ERROR: {}", err);
            err
        })?;
        
        println!("[download] Completed {} ({} bytes)", name, bytes.len());
        
        let hash = calculate_sha256(&file_path)?;
        hashes.insert(name.to_string(), hash);
    }
    
    // Download fake files
    for name in FAKE_FILES.iter() {
        current += 1;
        
        app.emit("download-progress", DownloadProgress {
            current,
            total: total_files,
            filename: name.to_string(),
            phase: "fake".to_string(),
        }).ok();
        
        let file_path = fake_dir.join(name);
        let url = format!("{}/{}", FAKE_FILES_BASE_URL, name);
        
        let response = client.get(&url)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        
        if !response.status().is_success() {
            app.emit("download-error", format!("Failed to download {}: HTTP {}", name, response.status())).ok();
            return Err(format!("Failed to download {}: HTTP {}", name, response.status()));
        }
        
        let bytes = response.bytes().await.map_err(|e| e.to_string())?;
        fs::write(&file_path, &bytes).map_err(|e| e.to_string())?;
    }
    
    // Download lists
    for name in LISTS.iter() {
        current += 1;
        
        app.emit("download-progress", DownloadProgress {
            current,
            total: total_files,
            filename: name.to_string(),
            phase: "lists".to_string(),
        }).ok();
        
        let file_path = lists_dir.join(name);
        let url = format!("{}/{}", LISTS_BASE_URL, name);
        
        let response = client.get(&url)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        
        if !response.status().is_success() {
            app.emit("download-error", format!("Failed to download {}: HTTP {}", name, response.status())).ok();
            return Err(format!("Failed to download {}: HTTP {}", name, response.status()));
        }
        
        let bytes = response.bytes().await.map_err(|e| e.to_string())?;
        fs::write(&file_path, &bytes).map_err(|e| e.to_string())?;
    }
    
    // Download filters
    for name in FILTERS.iter() {
        current += 1;
        
        app.emit("download-progress", DownloadProgress {
            current,
            total: total_files,
            filename: name.to_string(),
            phase: "filters".to_string(),
        }).ok();
        
        let file_path = filters_dir.join(name);
        let url = format!("{}/{}", FILTERS_BASE_URL, name);
        
        let response = client.get(&url)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        
        if !response.status().is_success() {
            app.emit("download-error", format!("Failed to download {}: HTTP {}", name, response.status())).ok();
            return Err(format!("Failed to download {}: HTTP {}", name, response.status()));
        }
        
        let bytes = response.bytes().await.map_err(|e| e.to_string())?;
        fs::write(&file_path, &bytes).map_err(|e| e.to_string())?;
    }
    
    save_stored_hashes(&hashes)?;
    
    // Emit download complete
    app.emit("download-complete", ()).ok();
    
    app.notification()
        .builder()
        .title("Готово")
        .body("Все файлы загружены")
        .show()
        .map_err(|e| e.to_string())?;
    
    Ok(())
}

#[tauri::command]
pub fn get_binary_path(filename: &str) -> String {
    get_zapret_dir().join(filename).to_string_lossy().to_string()
}

#[tauri::command]
pub fn get_winws_path() -> String {
    get_zapret_dir().join("winws.exe").to_string_lossy().to_string()
}

#[tauri::command]
pub fn get_filters_path() -> String {
    get_filters_dir().to_string_lossy().to_string()
}

#[tauri::command]
pub fn open_zapret_directory(app: AppHandle) -> Result<(), String> {
    let dir = get_zapret_dir();
    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    let path = dir.to_string_lossy().to_string();
    app.opener()
        .open_path(path, None::<&str>)
        .map_err(|e| e.to_string())
}