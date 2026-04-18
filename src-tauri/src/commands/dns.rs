use super::config::{get_managed_resources_dir, get_runtime_data_dir};
use dnsstamps::DoHBuilder;
use futures::future::join_all;
use reqwest::Client as HttpClient;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::{Instant, timeout};
use toml::{Table, Value};

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
const DOH_TEST_QUERY: &str = "AAABAAABAAAAAAAAB2V4YW1wbGUDY29tAAABAAE";

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
        if let Err(error) = QueryServiceStatus(service, &mut status) {
            let _ = CloseServiceHandle(service);
            let _ = CloseServiceHandle(manager);
            return Err(format!("Failed to query dnscrypt-proxy service: {error}"));
        }

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

fn push_rollback_error(
    rollback_errors: &mut Vec<String>,
    context: &str,
    result: Result<(), String>,
) {
    if let Err(error) = result {
        let message = format!("{context}: {error}");
        eprintln!("{message}");
        rollback_errors.push(message);
    }
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
    let mut content = Table::new();
    content.insert(
        "listen_addresses".to_string(),
        Value::Array(vec![
            Value::String("127.0.0.1:53".to_string()),
            Value::String("[::1]:53".to_string()),
        ]),
    );
    content.insert(
        "server_names".to_string(),
        Value::Array(
            static_entries
                .iter()
                .map(|(name, _)| Value::String(name.clone()))
                .collect(),
        ),
    );
    content.insert(
        "bootstrap_resolvers".to_string(),
        Value::Array(
            bootstrap_resolvers
                .iter()
                .map(|resolver| Value::String(resolver.clone()))
                .collect(),
        ),
    );
    content.insert("ignore_system_dns".to_string(), Value::Boolean(true));
    content.insert("cache".to_string(), Value::Boolean(true));
    content.insert("ipv4_servers".to_string(), Value::Boolean(true));
    content.insert("ipv6_servers".to_string(), Value::Boolean(true));
    content.insert("dnscrypt_servers".to_string(), Value::Boolean(false));
    content.insert("doh_servers".to_string(), Value::Boolean(true));
    content.insert("odoh_servers".to_string(), Value::Boolean(false));
    content.insert("require_dnssec".to_string(), Value::Boolean(false));
    content.insert("require_nolog".to_string(), Value::Boolean(false));
    content.insert("require_nofilter".to_string(), Value::Boolean(false));
    content.insert(
        "log_file".to_string(),
        Value::String(normalize_path_for_toml(&log_path)),
    );

    let mut static_table = Table::new();
    for (name, stamp) in static_entries {
        let mut entry = Table::new();
        entry.insert("stamp".to_string(), Value::String(stamp));
        static_table.insert(name, Value::Table(entry));
    }
    content.insert("static".to_string(), Value::Table(static_table));

    let content = toml::to_string_pretty(&content)
        .map_err(|error| format!("Не удалось сериализовать dnscrypt-proxy config: {error}"))?;

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
function Get-DnsTargetAdapters {
  $virtualPattern = 'TAP|WireGuard|OpenVPN|Hyper-V|vEthernet|Container|Loopback'
  $configs = @(
    Get-NetIPConfiguration | Where-Object {
      $_.NetAdapter -ne $null -and
      $_.NetAdapter.Status -eq 'Up' -and
      $_.IPv4DefaultGateway -ne $null -and
      $_.NetAdapter.InterfaceDescription -notmatch $virtualPattern -and
      $_.NetAdapter.Name -notmatch $virtualPattern
    } | Sort-Object { $_.NetAdapter.InterfaceMetric }, InterfaceIndex
  )

  if ($configs.Count -eq 0) {
    $configs = @(
      Get-NetIPConfiguration | Where-Object {
        $_.NetAdapter -ne $null -and
        $_.NetAdapter.Status -eq 'Up' -and
        $_.IPv4DefaultGateway -ne $null
      } | Sort-Object { $_.NetAdapter.InterfaceMetric }, InterfaceIndex
    )
  }

  @($configs | Group-Object InterfaceIndex | ForEach-Object { $_.Group[0] })
}

$entries = @()
foreach ($config in @(Get-DnsTargetAdapters)) {
  $index = [int]$config.InterfaceIndex
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

// Scope the local DNS redirect to the default-route adapter(s) only.
// This avoids overriding tunnel / virtual / VPN adapters and mirrors the backup scope.
fn apply_local_system_dns() -> Result<(), String> {
    let script = r#"
function Get-DnsTargetAdapters {
  $virtualPattern = 'TAP|WireGuard|OpenVPN|Hyper-V|vEthernet|Container|Loopback'
  $configs = @(
    Get-NetIPConfiguration | Where-Object {
      $_.NetAdapter -ne $null -and
      $_.NetAdapter.Status -eq 'Up' -and
      $_.IPv4DefaultGateway -ne $null -and
      $_.NetAdapter.InterfaceDescription -notmatch $virtualPattern -and
      $_.NetAdapter.Name -notmatch $virtualPattern
    } | Sort-Object { $_.NetAdapter.InterfaceMetric }, InterfaceIndex
  )

  if ($configs.Count -eq 0) {
    $configs = @(
      Get-NetIPConfiguration | Where-Object {
        $_.NetAdapter -ne $null -and
        $_.NetAdapter.Status -eq 'Up' -and
        $_.IPv4DefaultGateway -ne $null
      } | Sort-Object { $_.NetAdapter.InterfaceMetric }, InterfaceIndex
    )
  }

  @($configs | Group-Object InterfaceIndex | ForEach-Object { $_.Group[0] })
}

foreach ($config in @(Get-DnsTargetAdapters)) {
  $index = [int]$config.InterfaceIndex
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
function Get-DnsTargetAdapters {
  $virtualPattern = 'TAP|WireGuard|OpenVPN|Hyper-V|vEthernet|Container|Loopback'
  $configs = @(
    Get-NetIPConfiguration | Where-Object {
      $_.NetAdapter -ne $null -and
      $_.NetAdapter.Status -eq 'Up' -and
      $_.IPv4DefaultGateway -ne $null -and
      $_.NetAdapter.InterfaceDescription -notmatch $virtualPattern -and
      $_.NetAdapter.Name -notmatch $virtualPattern
    } | Sort-Object { $_.NetAdapter.InterfaceMetric }, InterfaceIndex
  )

  if ($configs.Count -eq 0) {
    $configs = @(
      Get-NetIPConfiguration | Where-Object {
        $_.NetAdapter -ne $null -and
        $_.NetAdapter.Status -eq 'Up' -and
        $_.IPv4DefaultGateway -ne $null
      } | Sort-Object { $_.NetAdapter.InterfaceMetric }, InterfaceIndex
    )
  }

  @($configs | Group-Object InterfaceIndex | ForEach-Object { $_.Group[0] })
}

foreach ($config in @(Get-DnsTargetAdapters)) {
  $index = [int]$config.InterfaceIndex
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
    let parsed = match Url::parse(&url) {
        Ok(parsed) => parsed,
        Err(error) => {
            return DnsLatencyResult {
                url,
                reachable: false,
                latency_ms: None,
                error: Some(format!("Не удалось извлечь хост из URL: {error}")),
            };
        }
    };

    let mut doh_url = parsed.clone();
    doh_url.query_pairs_mut().append_pair("dns", DOH_TEST_QUERY);
    let host = parsed.host_str().unwrap_or_default().to_string();
    let port = parsed.port_or_known_default().unwrap_or(443);
    let client = match HttpClient::builder()
        .timeout(Duration::from_secs(3))
        .build()
    {
        Ok(client) => client,
        Err(error) => {
            return DnsLatencyResult {
                url,
                reachable: false,
                latency_ms: None,
                error: Some(format!("Не удалось создать HTTP клиент: {error}")),
            };
        }
    };

    let start = Instant::now();
    let http_result = client
        .get(doh_url)
        .header("accept", "application/dns-message")
        .send()
        .await;

    match http_result {
        Ok(response) => {
            if !response.status().is_success() {
                let tcp_latency = timeout(
                    Duration::from_secs(3),
                    TcpStream::connect(format!("{host}:{port}")),
                )
                .await;

                return match tcp_latency {
                    Ok(Ok(_)) => DnsLatencyResult {
                        url,
                        reachable: true,
                        latency_ms: Some(start.elapsed().as_millis() as u64),
                        error: None,
                    },
                    Ok(Err(error)) => DnsLatencyResult {
                        url,
                        reachable: false,
                        latency_ms: None,
                        error: Some(format!(
                            "DoH ответил статусом {}, fallback TCP не удался: {error}",
                            response.status()
                        )),
                    },
                    Err(_) => DnsLatencyResult {
                        url,
                        reachable: false,
                        latency_ms: None,
                        error: Some(format!(
                            "DoH ответил статусом {}, fallback TCP превысил таймаут",
                            response.status()
                        )),
                    },
                };
            }

            match response.bytes().await {
                Ok(_) => DnsLatencyResult {
                    url,
                    reachable: true,
                    latency_ms: Some(start.elapsed().as_millis() as u64),
                    error: None,
                },
                Err(error) => DnsLatencyResult {
                    url,
                    reachable: false,
                    latency_ms: None,
                    error: Some(format!("Не удалось прочитать DoH ответ: {error}")),
                },
            }
        }
        Err(http_error) => {
            let tcp_latency = timeout(
                Duration::from_secs(3),
                TcpStream::connect(format!("{host}:{port}")),
            )
            .await;

            match tcp_latency {
                Ok(Ok(_)) => DnsLatencyResult {
                    url,
                    reachable: true,
                    latency_ms: Some(start.elapsed().as_millis() as u64),
                    error: None,
                },
                Ok(Err(tcp_error)) => DnsLatencyResult {
                    url,
                    reachable: false,
                    latency_ms: None,
                    error: Some(format!(
                        "DoH запрос не удался: {http_error}; fallback TCP не удался: {tcp_error}"
                    )),
                },
                Err(_) => DnsLatencyResult {
                    url,
                    reachable: false,
                    latency_ms: None,
                    error: Some(format!(
                        "DoH запрос не удался: {http_error}; fallback TCP превысил таймаут"
                    )),
                },
            }
        }
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
        let mut rollback_errors = Vec::new();
        push_rollback_error(
            &mut rollback_errors,
            "run_dns_proxy_action(uninstall)",
            run_dns_proxy_action(&config_path, "uninstall").map(|_| ()),
        );
        if rollback_errors.is_empty() {
            return Err(error);
        }

        return Err(format!(
            "{error}. Дополнительно не удалось откатить запуск: {}",
            rollback_errors.join(" | ")
        ));
    }

    if let Err(error) = apply_local_system_dns() {
        let mut rollback_errors = Vec::new();
        push_rollback_error(
            &mut rollback_errors,
            "restore_system_dns_from_backup",
            restore_system_dns_from_backup(&backup),
        );
        push_rollback_error(
            &mut rollback_errors,
            "run_dns_proxy_action(stop)",
            run_dns_proxy_action(&config_path, "stop").map(|_| ()),
        );
        push_rollback_error(
            &mut rollback_errors,
            "run_dns_proxy_action(uninstall)",
            run_dns_proxy_action(&config_path, "uninstall").map(|_| ()),
        );
        if rollback_errors.is_empty() {
            return Err(error);
        }

        return Err(format!(
            "{error}. Дополнительно не удалось откатить изменения: {}",
            rollback_errors.join(" | ")
        ));
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
    Ok(join_all(urls.into_iter().map(measure_dns_latency)).await)
}
