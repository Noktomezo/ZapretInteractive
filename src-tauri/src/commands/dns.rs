use super::config::{get_managed_resources_dir, get_runtime_data_dir};
use dnsstamps::DoHBuilder;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use surge_ping::{Client, Config, ICMP, PingIdentifier, PingSequence};
use tokio::net::lookup_host;

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
use windows::Win32::System::Services::{
    CloseServiceHandle, OpenSCManagerW, OpenServiceW, QueryServiceStatus, SC_MANAGER_CONNECT,
    SERVICE_QUERY_STATUS, SERVICE_STATUS, SERVICE_STOPPED,
};
#[cfg(windows)]
use windows::core::PCWSTR;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;
const DNSCRYPT_PROXY_SERVICE_NAME: &str = "dnscrypt-proxy";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsProxyStatus {
    installed: bool,
    running: bool,
    module_available: bool,
    config_path: String,
    service_name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsLatencyResult {
    url: String,
    reachable: bool,
    latency_ms: Option<u64>,
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DnsSystemBackup {
    adapters: Vec<DnsAdapterBackup>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DnsAdapterBackup {
    interface_index: u32,
    server_addresses: Vec<String>,
}

fn dns_proxy_module_dir() -> PathBuf {
    get_managed_resources_dir()
        .join("modules")
        .join("dnscrypt-proxy")
}

fn dns_proxy_binary_path() -> PathBuf {
    dns_proxy_module_dir().join("dnscrypt-proxy.exe")
}

fn dns_proxy_runtime_dir() -> PathBuf {
    get_runtime_data_dir().join("dnscrypt-proxy")
}

fn dns_proxy_config_path() -> PathBuf {
    dns_proxy_runtime_dir().join("dnscrypt-proxy.toml")
}

fn dns_proxy_log_path() -> PathBuf {
    dns_proxy_runtime_dir().join("dnscrypt-proxy.log")
}

fn dns_proxy_backup_path() -> PathBuf {
    dns_proxy_runtime_dir().join("system-dns-backup.json")
}

fn ensure_dns_proxy_runtime_dir() -> Result<PathBuf, String> {
    let dir = dns_proxy_runtime_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn normalize_path_for_toml(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn build_dns_proxy_status(installed: bool, running: bool) -> DnsProxyStatus {
    DnsProxyStatus {
        installed,
        running,
        module_available: dns_proxy_binary_path().is_file(),
        config_path: dns_proxy_config_path().to_string_lossy().to_string(),
        service_name: DNSCRYPT_PROXY_SERVICE_NAME.to_string(),
    }
}

#[cfg(windows)]
fn to_wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(windows)]
fn query_dns_proxy_service_state() -> Result<Option<bool>, String> {
    unsafe {
        let manager = OpenSCManagerW(None, None, SC_MANAGER_CONNECT)
            .map_err(|e| format!("Failed to open Service Control Manager: {e}"))?;
        let service_name = to_wide(DNSCRYPT_PROXY_SERVICE_NAME);
        let service = OpenServiceW(manager, PCWSTR(service_name.as_ptr()), SERVICE_QUERY_STATUS);

        let service = match service {
            Ok(service) => service,
            Err(_) => {
                let _ = CloseServiceHandle(manager);
                return Ok(None);
            }
        };

        let mut status = SERVICE_STATUS::default();
        QueryServiceStatus(service, &mut status)
            .map_err(|e| format!("Failed to query dnscrypt-proxy service: {e}"))?;

        let _ = CloseServiceHandle(service);
        let _ = CloseServiceHandle(manager);

        Ok(Some(status.dwCurrentState != SERVICE_STOPPED))
    }
}

#[cfg(not(windows))]
fn query_dns_proxy_service_state() -> Result<Option<bool>, String> {
    Ok(None)
}

fn get_dns_proxy_status_inner() -> Result<DnsProxyStatus, String> {
    let state = query_dns_proxy_service_state()?;
    Ok(build_dns_proxy_status(
        state.is_some(),
        state.unwrap_or(false),
    ))
}

fn run_command_capture(command: &mut Command, error_context: &str) -> Result<String, String> {
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);

    let output = command
        .output()
        .map_err(|e| format!("{error_context}: {e}"))?;

    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let detail = if !stderr.is_empty() {
        stderr
    } else if !stdout.is_empty() {
        stdout
    } else {
        format!("exit code {}", output.status)
    };
    Err(format!("{error_context}: {detail}"))
}

fn run_powershell(script: &str, error_context: &str) -> Result<String, String> {
    let mut command = Command::new("powershell");
    command.args([
        "-NoProfile",
        "-NonInteractive",
        "-ExecutionPolicy",
        "Bypass",
        "-Command",
        script,
    ]);
    run_command_capture(&mut command, error_context)
}

fn run_dns_proxy_action(config_path: &Path, action: &str) -> Result<String, String> {
    let mut command = Command::new(dns_proxy_binary_path());
    command.current_dir(dns_proxy_module_dir()).args([
        "-config",
        &config_path.to_string_lossy(),
        "-service",
        action,
    ]);
    run_command_capture(
        &mut command,
        &format!("Failed to {action} dnscrypt-proxy service"),
    )
}

fn check_dns_proxy_config(config_path: &Path) -> Result<(), String> {
    let mut command = Command::new(dns_proxy_binary_path());
    command.current_dir(dns_proxy_module_dir()).args([
        "-config",
        &config_path.to_string_lossy(),
        "-check",
    ]);
    run_command_capture(&mut command, "Failed to validate dnscrypt-proxy config")?;
    Ok(())
}

fn ensure_dns_proxy_binary_available() -> Result<(), String> {
    if dns_proxy_binary_path().is_file() {
        Ok(())
    } else {
        Err("dnscrypt-proxy.exe не найден в resources/modules/dnscrypt-proxy".to_string())
    }
}

fn normalize_bootstrap_resolvers(resolvers: Vec<String>) -> Result<Vec<String>, String> {
    let mut normalized = Vec::new();

    for resolver in resolvers {
        let trimmed = resolver.trim();
        if trimmed.is_empty() {
            continue;
        }

        let ip = trimmed
            .parse::<std::net::IpAddr>()
            .map_err(|_| format!("Некорректный bootstrap resolver: {trimmed}"))?;

        normalized.push(match ip {
            std::net::IpAddr::V4(addr) => format!("{addr}:53"),
            std::net::IpAddr::V6(addr) => format!("[{addr}]:53"),
        });
    }

    if normalized.is_empty() {
        return Err("Нужен хотя бы один bootstrap resolver".to_string());
    }

    normalized.sort();
    normalized.dedup();
    Ok(normalized)
}

fn build_static_stamp(url: &str) -> Result<(String, String), String> {
    let parsed = Url::parse(url).map_err(|e| format!("Некорректный DoH URL '{url}': {e}"))?;
    if parsed.scheme() != "https" {
        return Err(format!("DoH URL должен быть https: {url}"));
    }

    let host = parsed
        .host_str()
        .ok_or_else(|| format!("DoH URL без хоста: {url}"))?;
    let port = parsed.port_or_known_default().unwrap_or(443);
    let mut path = parsed.path().to_string();
    if path.is_empty() {
        path = "/dns-query".to_string();
    }
    if let Some(query) = parsed.query() {
        path.push('?');
        path.push_str(query);
    }

    let stamp = DoHBuilder::new(host.to_string(), path)
        .with_port(port)
        .serialize()
        .map_err(|e| format!("Не удалось собрать stamp для '{url}': {e}"))?;

    Ok((host.to_string(), stamp))
}

fn write_dns_proxy_config(
    doh_urls: Vec<String>,
    bootstrap_resolvers: Vec<String>,
) -> Result<PathBuf, String> {
    ensure_dns_proxy_runtime_dir()?;

    let static_entries = doh_urls
        .iter()
        .enumerate()
        .map(|(index, url)| {
            let (_, stamp) = build_static_stamp(url)?;
            Ok((format!("custom-{:02}", index + 1), stamp))
        })
        .collect::<Result<Vec<_>, String>>()?;

    if static_entries.is_empty() {
        return Err("Нужен хотя бы один DoH endpoint".to_string());
    }

    let config_path = dns_proxy_config_path();
    let log_path = dns_proxy_log_path();
    let server_names = static_entries
        .iter()
        .map(|(name, _)| format!("'{name}'"))
        .collect::<Vec<_>>()
        .join(", ");
    let bootstrap = bootstrap_resolvers
        .iter()
        .map(|resolver| format!("'{resolver}'"))
        .collect::<Vec<_>>()
        .join(", ");

    let mut content = String::new();
    content.push_str("listen_addresses = ['127.0.0.1:53', '[::1]:53']\n");
    content.push_str(&format!("server_names = [{server_names}]\n"));
    content.push_str(&format!("bootstrap_resolvers = [{bootstrap}]\n"));
    content.push_str("ignore_system_dns = true\n");
    content.push_str("cache = true\n");
    content.push_str("ipv4_servers = true\n");
    content.push_str("ipv6_servers = true\n");
    content.push_str("dnscrypt_servers = false\n");
    content.push_str("doh_servers = true\n");
    content.push_str("odoh_servers = false\n");
    content.push_str("require_dnssec = false\n");
    content.push_str("require_nolog = false\n");
    content.push_str("require_nofilter = false\n");
    content.push_str(&format!(
        "log_file = '{}'\n\n",
        normalize_path_for_toml(&log_path)
    ));

    for (name, stamp) in static_entries {
        content.push_str(&format!("[static.'{name}']\n"));
        content.push_str(&format!("stamp = '{stamp}'\n\n"));
    }

    fs::write(&config_path, content).map_err(|e| e.to_string())?;
    Ok(config_path)
}

fn read_dns_backup() -> Result<Option<DnsSystemBackup>, String> {
    let backup_path = dns_proxy_backup_path();
    if !backup_path.is_file() {
        return Ok(None);
    }

    let content = fs::read_to_string(&backup_path).map_err(|e| e.to_string())?;
    let backup = serde_json::from_str::<DnsSystemBackup>(&content).map_err(|e| e.to_string())?;
    Ok(Some(backup))
}

fn write_dns_backup(backup: &DnsSystemBackup) -> Result<(), String> {
    ensure_dns_proxy_runtime_dir()?;
    let content = serde_json::to_string_pretty(backup).map_err(|e| e.to_string())?;
    fs::write(dns_proxy_backup_path(), content).map_err(|e| e.to_string())
}

fn remove_dns_backup() {
    let _ = fs::remove_file(dns_proxy_backup_path());
}

fn capture_current_system_dns() -> Result<DnsSystemBackup, String> {
    let script = r#"
$indices = @(Get-NetAdapter | Where-Object {
  $_.Status -eq 'Up' -and $_.Name -notmatch 'Loopback' -and $_.InterfaceDescription -notmatch 'Loopback'
} | Select-Object -ExpandProperty ifIndex)
$entries = @()
foreach ($index in $indices) {
  $servers = @(Get-DnsClientServerAddress -InterfaceIndex $index -ErrorAction SilentlyContinue |
    ForEach-Object { $_.ServerAddresses } |
    Where-Object { $_ -and $_.Trim().Length -gt 0 } |
    Select-Object -Unique)
  $entries += [PSCustomObject]@{
    interfaceIndex = [int]$index
    serverAddresses = @($servers)
  }
}
@($entries) | ConvertTo-Json -Compress
"#;

    let output = run_powershell(script, "Не удалось получить текущие системные DNS")?;
    let adapters = if output.trim().is_empty() {
        Vec::new()
    } else {
        serde_json::from_str::<Vec<DnsAdapterBackup>>(&output).map_err(|e| e.to_string())?
    };

    Ok(DnsSystemBackup { adapters })
}

fn apply_local_system_dns() -> Result<(), String> {
    let script = r#"
$indices = @(Get-NetAdapter | Where-Object {
  $_.Status -eq 'Up' -and $_.Name -notmatch 'Loopback' -and $_.InterfaceDescription -notmatch 'Loopback'
} | Select-Object -ExpandProperty ifIndex)
foreach ($index in $indices) {
  Set-DnsClientServerAddress -InterfaceIndex $index -ServerAddresses @('127.0.0.1', '::1') -ErrorAction Stop
}
"#;

    run_powershell(
        script,
        "Не удалось переключить системный DNS на dnscrypt-proxy",
    )?;
    Ok(())
}

fn restore_system_dns_from_backup(backup: &DnsSystemBackup) -> Result<(), String> {
    let backup_path = dns_proxy_backup_path();
    write_dns_backup(backup)?;
    let normalized_path = normalize_path_for_toml(&backup_path);
    let script = format!(
        r#"
$data = Get-Content -Raw -Path '{normalized_path}' | ConvertFrom-Json
$errors = @()
foreach ($entry in @($data.adapters)) {{
  $adapter = Get-NetAdapter -InterfaceIndex $entry.interfaceIndex -ErrorAction SilentlyContinue
  if ($null -eq $adapter) {{
    continue
  }}
  $servers = @($entry.serverAddresses | Where-Object {{ $_ -and $_.Trim().Length -gt 0 }})
  try {{
    if ($servers.Count -gt 0) {{
      Set-DnsClientServerAddress -InterfaceIndex $entry.interfaceIndex -ServerAddresses $servers -ErrorAction Stop
    }} else {{
      Set-DnsClientServerAddress -InterfaceIndex $entry.interfaceIndex -ResetServerAddresses -ErrorAction Stop
    }}
  }} catch {{
    $errors += $_.Exception.Message
  }}
}}
if ($errors.Count -gt 0) {{
  throw ($errors -join '; ')
}}
"#
    );

    run_powershell(&script, "Не удалось восстановить системные DNS адреса")?;
    Ok(())
}

fn reset_system_dns_for_active_adapters() -> Result<(), String> {
    let script = r#"
$indices = @(Get-NetAdapter | Where-Object {
  $_.Status -eq 'Up' -and $_.Name -notmatch 'Loopback' -and $_.InterfaceDescription -notmatch 'Loopback'
} | Select-Object -ExpandProperty ifIndex)
foreach ($index in $indices) {
  Set-DnsClientServerAddress -InterfaceIndex $index -ResetServerAddresses -ErrorAction Stop
}
"#;

    run_powershell(
        script,
        "Не удалось сбросить системный DNS к настройкам адаптера",
    )?;
    Ok(())
}

async fn measure_dns_latency(url: String) -> DnsLatencyResult {
    let host = match Url::parse(&url)
        .ok()
        .and_then(|parsed| parsed.host_str().map(str::to_string))
    {
        Some(host) => host,
        None => {
            return DnsLatencyResult {
                url,
                reachable: false,
                latency_ms: None,
                error: Some("Не удалось извлечь хост из URL".to_string()),
            };
        }
    };

    let target = match lookup_host(format!("{host}:0"))
        .await
        .map_err(|error| error.to_string())
        .and_then(|mut entries| {
            entries
                .next()
                .ok_or_else(|| "Хост не вернул ни одного IP-адреса".to_string())
        }) {
        Ok(target) => target,
        Err(error) => {
            return DnsLatencyResult {
                url,
                reachable: false,
                latency_ms: None,
                error: Some(error),
            };
        }
    };

    let mut config_builder = Config::builder();
    if target.is_ipv6() {
        config_builder = config_builder.kind(ICMP::V6);
    }

    let client = match Client::new(&config_builder.build()) {
        Ok(client) => client,
        Err(error) => {
            return DnsLatencyResult {
                url,
                reachable: false,
                latency_ms: None,
                error: Some(error.to_string()),
            };
        }
    };

    let mut pinger = client.pinger(target.ip(), PingIdentifier(0)).await;
    if let SocketAddr::V6(addr) = target {
        pinger.scope_id(addr.scope_id());
    }
    pinger.timeout(Duration::from_secs(1));

    match pinger.ping(PingSequence(0), &[0; 8]).await {
        Ok((_, rtt)) => DnsLatencyResult {
            url,
            reachable: true,
            latency_ms: Some(rtt.as_millis() as u64),
            error: None,
        },
        Err(error) => DnsLatencyResult {
            url,
            reachable: false,
            latency_ms: None,
            error: Some(error.to_string()),
        },
    }
}

#[tauri::command]
pub fn get_dns_proxy_status() -> Result<DnsProxyStatus, String> {
    ensure_dns_proxy_runtime_dir()?;
    get_dns_proxy_status_inner()
}

fn start_dns_proxy_inner(
    doh_urls: Vec<String>,
    bootstrap_resolvers: Vec<String>,
) -> Result<DnsProxyStatus, String> {
    ensure_dns_proxy_binary_available()?;
    let normalized_bootstrap_resolvers = normalize_bootstrap_resolvers(bootstrap_resolvers)?;
    let config_path = write_dns_proxy_config(doh_urls, normalized_bootstrap_resolvers)?;
    check_dns_proxy_config(&config_path)?;

    let backup = if let Some(existing) = read_dns_backup()? {
        existing
    } else {
        let captured = capture_current_system_dns()?;
        write_dns_backup(&captured)?;
        captured
    };

    let _ = run_dns_proxy_action(&config_path, "stop");
    let _ = run_dns_proxy_action(&config_path, "uninstall");

    run_dns_proxy_action(&config_path, "install")?;
    if let Err(error) = run_dns_proxy_action(&config_path, "start") {
        let _ = run_dns_proxy_action(&config_path, "uninstall");
        return Err(error);
    }

    if let Err(error) = apply_local_system_dns() {
        let _ = restore_system_dns_from_backup(&backup);
        let _ = run_dns_proxy_action(&config_path, "stop");
        let _ = run_dns_proxy_action(&config_path, "uninstall");
        return Err(error);
    }

    std::thread::sleep(std::time::Duration::from_millis(350));
    get_dns_proxy_status_inner()
}

fn stop_dns_proxy_inner() -> Result<DnsProxyStatus, String> {
    if let Some(backup) = read_dns_backup()? {
        restore_system_dns_from_backup(&backup)?;
        remove_dns_backup();
    } else {
        let _ = reset_system_dns_for_active_adapters();
    }

    let config_path = dns_proxy_config_path();
    let _ = run_dns_proxy_action(&config_path, "stop");
    let _ = run_dns_proxy_action(&config_path, "uninstall");

    std::thread::sleep(std::time::Duration::from_millis(250));
    get_dns_proxy_status_inner()
}

#[tauri::command]
pub async fn start_dns_proxy(
    doh_urls: Vec<String>,
    bootstrap_resolvers: Vec<String>,
) -> Result<DnsProxyStatus, String> {
    tauri::async_runtime::spawn_blocking(move || {
        start_dns_proxy_inner(doh_urls, bootstrap_resolvers)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn stop_dns_proxy() -> Result<DnsProxyStatus, String> {
    tauri::async_runtime::spawn_blocking(stop_dns_proxy_inner)
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn check_dns_provider_latency(
    urls: Vec<String>,
) -> Result<Vec<DnsLatencyResult>, String> {
    let mut results = Vec::with_capacity(urls.len());
    for url in urls {
        results.push(measure_dns_latency(url).await);
    }
    Ok(results)
}
