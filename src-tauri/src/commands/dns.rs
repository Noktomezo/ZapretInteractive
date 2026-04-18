use super::config::{get_managed_resources_dir, get_runtime_data_dir};
#[cfg(windows)]
use crate::get_windows_build_number;
use dnsstamps::DoHBuilder;
use duct::{Expression, cmd};
use futures::future::join_all;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::fs;
use std::net::IpAddr;
#[cfg(windows)]
use std::net::Ipv4Addr;
use std::path::{Path, PathBuf};
use std::time::Duration;
use surge_ping::ping as icmp_ping;
use tokio::net::{TcpStream, lookup_host};
use tokio::time::{Instant, timeout};
use toml::{Table, Value};

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
use windows::Win32::Foundation::{ERROR_BUFFER_OVERFLOW, WIN32_ERROR};
#[cfg(windows)]
use windows::Win32::NetworkManagement::IpHelper::{
    ConvertInterfaceIndexToLuid, ConvertInterfaceLuidToGuid, DNS_INTERFACE_SETTINGS,
    DNS_INTERFACE_SETTINGS_VERSION1, DNS_SETTING_IPV6, DNS_SETTING_NAMESERVER,
    DNS_SETTING_PROFILE_NAMESERVER, FreeInterfaceDnsSettings, GAA_FLAG_INCLUDE_GATEWAYS,
    GET_ADAPTERS_ADDRESSES_FLAGS, GetAdaptersAddresses, GetInterfaceDnsSettings,
    IP_ADAPTER_ADDRESSES_LH, SetInterfaceDnsSettings,
};
#[cfg(windows)]
use windows::Win32::NetworkManagement::Ndis::{IfOperStatusUp, NET_LUID_LH};
#[cfg(windows)]
use windows::Win32::Networking::WinSock::AF_UNSPEC;
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
#[cfg(windows)]
const NATIVE_INTERFACE_DNS_MIN_BUILD: u32 = 19041;
#[cfg(windows)]
const ADAPTER_EXCLUDE_PATTERNS: &[&str] = &[
    "TAP",
    "WIREGUARD",
    "OPENVPN",
    "HYPER-V",
    "VETHERNET",
    "CONTAINER",
    "LOOPBACK",
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsProxyStatus {
    installed: bool,
    running: bool,
    app_managed: bool,
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
    #[serde(
        default,
        rename = "capturedAtMs",
        skip_serializing_if = "Option::is_none"
    )]
    captured_at_ms: Option<u64>,
    adapters: Vec<DnsAdapterBackup>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DnsAdapterBackup {
    interface_index: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    server_addresses: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    ipv4_server_addresses: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    ipv6_server_addresses: Vec<String>,
}

#[cfg(windows)]
#[derive(Clone)]
struct TargetAdapter {
    interface_index: u32,
    luid: NET_LUID_LH,
    ipv4_metric: u32,
    friendly_name: String,
    description: String,
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

fn dns_proxy_managed_marker_path() -> PathBuf {
    dns_proxy_runtime_dir().join("managed-by-app.marker")
}

fn ensure_dns_proxy_runtime_dir() -> Result<PathBuf, String> {
    let dir = dns_proxy_runtime_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn is_dns_proxy_app_managed() -> bool {
    dns_proxy_managed_marker_path().is_file()
}

fn write_dns_managed_marker() -> Result<(), String> {
    ensure_dns_proxy_runtime_dir()?;
    fs::write(dns_proxy_managed_marker_path(), []).map_err(|e| e.to_string())
}

fn remove_dns_managed_marker() {
    let _ = fs::remove_file(dns_proxy_managed_marker_path());
}

fn normalize_path_for_toml(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(windows)]
fn supports_native_interface_dns_api() -> bool {
    get_windows_build_number().is_some_and(|build| build >= NATIVE_INTERFACE_DNS_MIN_BUILD)
}

#[cfg(not(windows))]
fn supports_native_interface_dns_api() -> bool {
    false
}

impl DnsAdapterBackup {
    fn from_servers(interface_index: u32, servers: Vec<String>) -> Self {
        let (ipv4_server_addresses, ipv6_server_addresses): (Vec<_>, Vec<_>) = servers
            .into_iter()
            .partition(|server| server.parse::<Ipv4Addr>().is_ok());

        Self {
            interface_index,
            server_addresses: Vec::new(),
            ipv4_server_addresses,
            ipv6_server_addresses,
        }
    }

    fn effective_ipv4_servers(&self) -> Vec<String> {
        if !self.ipv4_server_addresses.is_empty() || !self.ipv6_server_addresses.is_empty() {
            return self.ipv4_server_addresses.clone();
        }

        self.server_addresses
            .iter()
            .filter(|server| server.parse::<Ipv4Addr>().is_ok())
            .cloned()
            .collect()
    }

    fn effective_ipv6_servers(&self) -> Vec<String> {
        if !self.ipv4_server_addresses.is_empty() || !self.ipv6_server_addresses.is_empty() {
            return self.ipv6_server_addresses.clone();
        }

        self.server_addresses
            .iter()
            .filter(|server| server.parse::<std::net::Ipv6Addr>().is_ok())
            .cloned()
            .collect()
    }

    fn effective_servers(&self) -> Vec<String> {
        let mut servers = self.effective_ipv4_servers();
        servers.extend(self.effective_ipv6_servers());
        servers
    }
}

impl DnsSystemBackup {
    fn new(adapters: Vec<DnsAdapterBackup>) -> Self {
        Self {
            captured_at_ms: Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
            ),
            adapters,
        }
    }
}

fn is_loopback_dns_server(server: &str) -> bool {
    matches!(server.trim(), "127.0.0.1" | "::1")
}

fn backup_has_only_loopback_dns(backup: &DnsSystemBackup) -> bool {
    let all_servers = backup
        .adapters
        .iter()
        .flat_map(DnsAdapterBackup::effective_servers)
        .collect::<Vec<_>>();

    !all_servers.is_empty()
        && all_servers
            .iter()
            .all(|server| is_loopback_dns_server(server))
}

#[cfg(windows)]
fn wide_ptr_to_string(ptr: windows::core::PWSTR) -> String {
    if ptr.is_null() {
        return String::new();
    }

    unsafe {
        let mut len = 0usize;
        while *ptr.0.add(len) != 0 {
            len += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(ptr.0, len))
    }
}

#[cfg(windows)]
fn parse_dns_server_list(raw: &str) -> Vec<String> {
    raw.split([',', ' '])
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(windows)]
fn matches_adapter_exclude_pattern(value: &str) -> bool {
    let upper = value.to_ascii_uppercase();
    ADAPTER_EXCLUDE_PATTERNS
        .iter()
        .any(|pattern| upper.contains(pattern))
}

#[cfg(windows)]
fn adapter_has_default_gateway(adapter: &IP_ADAPTER_ADDRESSES_LH) -> bool {
    !adapter.FirstGatewayAddress.is_null()
}

#[cfg(windows)]
fn adapter_is_preferred(
    adapter: &IP_ADAPTER_ADDRESSES_LH,
    friendly_name: &str,
    description: &str,
) -> bool {
    adapter.OperStatus == IfOperStatusUp
        && adapter_has_default_gateway(adapter)
        && !matches_adapter_exclude_pattern(friendly_name)
        && !matches_adapter_exclude_pattern(description)
}

#[cfg(windows)]
fn adapter_is_fallback(adapter: &IP_ADAPTER_ADDRESSES_LH) -> bool {
    adapter.OperStatus == IfOperStatusUp && adapter_has_default_gateway(adapter)
}

#[cfg(windows)]
fn enumerate_target_adapters() -> Result<Vec<TargetAdapter>, String> {
    unsafe {
        let mut buffer_len = 15_000u32;
        let mut buffer = vec![0u8; buffer_len as usize];
        let flags = GET_ADAPTERS_ADDRESSES_FLAGS(GAA_FLAG_INCLUDE_GATEWAYS.0);
        let mut result = GetAdaptersAddresses(
            AF_UNSPEC.0 as u32,
            flags,
            None,
            Some(buffer.as_mut_ptr().cast::<IP_ADAPTER_ADDRESSES_LH>()),
            &mut buffer_len,
        );

        if result == ERROR_BUFFER_OVERFLOW.0 {
            buffer.resize(buffer_len as usize, 0);
            result = GetAdaptersAddresses(
                AF_UNSPEC.0 as u32,
                flags,
                None,
                Some(buffer.as_mut_ptr().cast::<IP_ADAPTER_ADDRESSES_LH>()),
                &mut buffer_len,
            );
        }

        if result != 0 {
            return Err(format!("GetAdaptersAddresses failed: {result}"));
        }

        let mut preferred = Vec::new();
        let mut fallback = Vec::new();
        let mut current = buffer.as_mut_ptr().cast::<IP_ADAPTER_ADDRESSES_LH>();
        while !current.is_null() {
            let adapter = &*current;
            let interface_index = adapter.Anonymous1.Anonymous.IfIndex;
            if interface_index == 0 {
                current = adapter.Next;
                continue;
            }

            let friendly_name = wide_ptr_to_string(adapter.FriendlyName);
            let description = wide_ptr_to_string(adapter.Description);
            let target = TargetAdapter {
                interface_index,
                luid: adapter.Luid,
                ipv4_metric: adapter.Ipv4Metric,
                friendly_name,
                description,
            };

            if adapter_is_preferred(adapter, &target.friendly_name, &target.description) {
                preferred.push(target.clone());
            }

            if adapter_is_fallback(adapter) {
                fallback.push(target);
            }

            current = adapter.Next;
        }

        let mut adapters = if preferred.is_empty() {
            fallback
        } else {
            preferred
        };

        adapters.sort_by_key(|adapter| (adapter.ipv4_metric, adapter.interface_index));
        adapters.dedup_by_key(|adapter| adapter.interface_index);
        Ok(adapters)
    }
}

#[cfg(windows)]
fn get_interface_guid_by_index(interface_index: u32) -> Result<windows::core::GUID, String> {
    unsafe {
        let mut luid = NET_LUID_LH::default();
        let result = ConvertInterfaceIndexToLuid(interface_index, &mut luid);
        if result != WIN32_ERROR(0) {
            return Err(format!(
                "ConvertInterfaceIndexToLuid({interface_index}) failed: {result:?}"
            ));
        }

        let mut guid = windows::core::GUID::zeroed();
        let result = ConvertInterfaceLuidToGuid(&luid, &mut guid);
        if result != WIN32_ERROR(0) {
            return Err(format!(
                "ConvertInterfaceLuidToGuid({interface_index}) failed: {result:?}"
            ));
        }
        Ok(guid)
    }
}

#[cfg(windows)]
fn get_interface_guid(adapter: &TargetAdapter) -> Result<windows::core::GUID, String> {
    unsafe {
        let mut guid = windows::core::GUID::zeroed();
        let result = ConvertInterfaceLuidToGuid(&adapter.luid, &mut guid);
        if result != WIN32_ERROR(0) {
            return Err(format!(
                "ConvertInterfaceLuidToGuid({}) failed: {result:?}",
                adapter.interface_index
            ));
        }
        Ok(guid)
    }
}

#[cfg(windows)]
fn get_configured_dns_servers(interface_guid: windows::core::GUID) -> Result<Vec<String>, String> {
    unsafe {
        let mut settings = DNS_INTERFACE_SETTINGS {
            Version: DNS_INTERFACE_SETTINGS_VERSION1,
            ..Default::default()
        };
        let result = GetInterfaceDnsSettings(interface_guid, &mut settings);
        if result != WIN32_ERROR(0) {
            return Err(format!("GetInterfaceDnsSettings failed: {result:?}"));
        }

        let mut servers = parse_dns_server_list(&wide_ptr_to_string(settings.NameServer));
        if servers.is_empty() {
            servers = parse_dns_server_list(&wide_ptr_to_string(settings.ProfileNameServer));
        }
        FreeInterfaceDnsSettings(&mut settings);
        Ok(servers)
    }
}

#[cfg(windows)]
fn apply_interface_dns_family(
    interface_guid: windows::core::GUID,
    servers: &[String],
    ipv6: bool,
) -> Result<(), String> {
    unsafe {
        let value = servers.join(" ");
        let mut value_wide = to_wide(&value);
        let mut profile_value_wide = to_wide(&value);
        let flags = if ipv6 {
            DNS_SETTING_NAMESERVER | DNS_SETTING_PROFILE_NAMESERVER | DNS_SETTING_IPV6
        } else {
            DNS_SETTING_NAMESERVER | DNS_SETTING_PROFILE_NAMESERVER
        };
        let settings = DNS_INTERFACE_SETTINGS {
            Version: DNS_INTERFACE_SETTINGS_VERSION1,
            Flags: flags as u64,
            NameServer: windows::core::PWSTR(value_wide.as_mut_ptr()),
            ProfileNameServer: windows::core::PWSTR(profile_value_wide.as_mut_ptr()),
            ..Default::default()
        };
        let result = SetInterfaceDnsSettings(interface_guid, &settings);
        if result != WIN32_ERROR(0) {
            let family = if ipv6 { "IPv6" } else { "IPv4" };
            return Err(format!(
                "SetInterfaceDnsSettings({family}) failed: {result:?}"
            ));
        }
    }

    Ok(())
}

#[cfg(windows)]
fn apply_interface_dns_servers(
    interface_guid: windows::core::GUID,
    ipv4_servers: &[String],
    ipv6_servers: &[String],
) -> Result<(), String> {
    apply_interface_dns_family(interface_guid, ipv4_servers, false)?;
    apply_interface_dns_family(interface_guid, ipv6_servers, true)?;
    Ok(())
}

#[cfg(windows)]
fn capture_current_system_dns_native() -> Result<DnsSystemBackup, String> {
    let adapters = enumerate_target_adapters()?;
    let adapters = adapters
        .into_iter()
        .map(|adapter| {
            let guid = get_interface_guid(&adapter)?;
            Ok(DnsAdapterBackup::from_servers(
                adapter.interface_index,
                get_configured_dns_servers(guid)?,
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(DnsSystemBackup::new(adapters))
}

#[cfg(windows)]
fn apply_local_system_dns_native() -> Result<(), String> {
    for adapter in enumerate_target_adapters()? {
        let guid = get_interface_guid(&adapter)?;
        apply_interface_dns_servers(guid, &["127.0.0.1".to_string()], &["::1".to_string()])?;
    }
    Ok(())
}

#[cfg(windows)]
fn restore_system_dns_from_backup_native(backup: &DnsSystemBackup) -> Result<(), String> {
    let mut errors = Vec::new();
    for adapter in &backup.adapters {
        let ipv4_servers = adapter.effective_ipv4_servers();
        let ipv6_servers = adapter.effective_ipv6_servers();
        match get_interface_guid_by_index(adapter.interface_index)
            .and_then(|guid| apply_interface_dns_servers(guid, &ipv4_servers, &ipv6_servers))
        {
            Ok(()) => {}
            Err(error) => errors.push(format!("{}: {error}", adapter.interface_index)),
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

fn build_dns_proxy_status(installed: bool, running: bool) -> DnsProxyStatus {
    DnsProxyStatus {
        installed,
        running,
        app_managed: running && is_dns_proxy_app_managed(),
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
    let running = state.unwrap_or(false);
    if !running {
        remove_dns_managed_marker();
    }
    Ok(build_dns_proxy_status(state.is_some(), running))
}

#[cfg(windows)]
fn configure_expression(expression: Expression) -> Expression {
    expression.before_spawn(|command| {
        command.creation_flags(CREATE_NO_WINDOW);
        Ok(())
    })
}

#[cfg(not(windows))]
fn configure_expression(expression: Expression) -> Expression {
    expression
}

fn run_command_capture(
    program: &str,
    args: Vec<String>,
    current_dir: Option<&Path>,
    error_context: &str,
) -> Result<String, String> {
    let mut expression = cmd(program, args)
        .stdout_capture()
        .stderr_capture()
        .unchecked();
    if let Some(dir) = current_dir {
        expression = expression.dir(dir);
    }
    let output = configure_expression(expression)
        .run()
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

fn run_dns_proxy_action(config_path: &Path, action: &str) -> Result<String, String> {
    let program = dns_proxy_binary_path().to_string_lossy().into_owned();
    let module_dir = dns_proxy_module_dir();
    run_command_capture(
        &program,
        vec![
            "-config".to_string(),
            config_path.to_string_lossy().into_owned(),
            "-service".to_string(),
            action.to_string(),
        ],
        Some(&module_dir),
        &format!("Failed to {action} dnscrypt-proxy service"),
    )
}

fn check_dns_proxy_config(config_path: &Path) -> Result<(), String> {
    let program = dns_proxy_binary_path().to_string_lossy().into_owned();
    let module_dir = dns_proxy_module_dir();
    run_command_capture(
        &program,
        vec![
            "-config".to_string(),
            config_path.to_string_lossy().into_owned(),
            "-check".to_string(),
        ],
        Some(&module_dir),
        "Failed to validate dnscrypt-proxy config",
    )?;
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

fn get_reusable_dns_backup() -> Result<Option<DnsSystemBackup>, String> {
    let Some(existing_backup) = read_dns_backup()? else {
        return Ok(None);
    };

    if !backup_has_only_loopback_dns(&existing_backup) {
        return Ok(Some(existing_backup));
    }

    let captured_backup = capture_current_system_dns()?;
    if backup_has_only_loopback_dns(&captured_backup) {
        return Err(
            "Резервная копия системного DNS содержит только loopback-адреса, восстановление небезопасно"
                .to_string(),
        );
    }

    write_dns_backup(&captured_backup)?;
    Ok(Some(captured_backup))
}

fn write_dns_backup(backup: &DnsSystemBackup) -> Result<(), String> {
    ensure_dns_proxy_runtime_dir()?;
    let content = serde_json::to_string_pretty(backup).map_err(|e| e.to_string())?;
    fs::write(dns_proxy_backup_path(), content).map_err(|e| e.to_string())
}

fn remove_dns_backup() {
    let _ = fs::remove_file(dns_proxy_backup_path());
}

fn capture_fresh_dns_backup() -> Result<DnsSystemBackup, String> {
    let captured = capture_current_system_dns()?;
    if backup_has_only_loopback_dns(&captured) {
        return Err(
            "Не удалось сохранить резервную копию системного DNS: обнаружены только loopback-адреса"
                .to_string(),
        );
    }

    write_dns_backup(&captured)?;
    Ok(captured)
}

fn cleanup_failed_dns_start(was_app_managed: bool) {
    if was_app_managed {
        return;
    }

    remove_dns_managed_marker();

    if !query_dns_proxy_service_state()
        .ok()
        .flatten()
        .unwrap_or(false)
    {
        remove_dns_backup();
    }
}

fn capture_current_system_dns() -> Result<DnsSystemBackup, String> {
    #[cfg(windows)]
    if supports_native_interface_dns_api() {
        return capture_current_system_dns_native();
    }

    Err("Нативное управление DNS требует Windows 10/11 build 19041 или новее".to_string())
}

fn apply_local_system_dns() -> Result<(), String> {
    #[cfg(windows)]
    if supports_native_interface_dns_api() {
        return apply_local_system_dns_native();
    }

    Err("Нативное управление DNS требует Windows 10/11 build 19041 или новее".to_string())
}

fn restore_system_dns_from_backup(backup: &DnsSystemBackup) -> Result<(), String> {
    #[cfg(windows)]
    if supports_native_interface_dns_api() {
        return restore_system_dns_from_backup_native(backup);
    }

    Err("Нативное управление DNS требует Windows 10/11 build 19041 или новее".to_string())
}

fn reset_system_dns_for_active_adapters() -> Result<(), String> {
    #[cfg(windows)]
    if supports_native_interface_dns_api() {
        let empty_backup = DnsSystemBackup {
            captured_at_ms: None,
            adapters: enumerate_target_adapters()?
                .into_iter()
                .map(|adapter| DnsAdapterBackup {
                    interface_index: adapter.interface_index,
                    server_addresses: Vec::new(),
                    ipv4_server_addresses: Vec::new(),
                    ipv6_server_addresses: Vec::new(),
                })
                .collect(),
        };
        return restore_system_dns_from_backup_native(&empty_backup);
    }

    Err("Нативное управление DNS требует Windows 10/11 build 19041 или новее".to_string())
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

    let host = parsed.host_str().unwrap_or_default().to_string();
    let port = parsed.port_or_known_default().unwrap_or(443);
    let resolved_ip = match lookup_host((host.as_str(), port)).await {
        Ok(mut addresses) => addresses
            .find(|address| matches!(address.ip(), IpAddr::V4(_) | IpAddr::V6(_)))
            .map(|address| address.ip()),
        Err(_) => None,
    };

    if let Some(ip) = resolved_ip {
        let payload = [0_u8; 8];
        let icmp_result = timeout(Duration::from_millis(1500), icmp_ping(ip, &payload)).await;

        match icmp_result {
            Ok(Ok((_packet, duration))) => {
                return DnsLatencyResult {
                    url,
                    reachable: true,
                    latency_ms: Some(duration.as_millis() as u64),
                    error: None,
                };
            }
            Ok(Err(icmp_error)) => {
                let tcp_started_at = Instant::now();
                let tcp_result = timeout(
                    Duration::from_millis(1500),
                    TcpStream::connect(format!("{host}:{port}")),
                )
                .await;

                return match tcp_result {
                    Ok(Ok(_)) => DnsLatencyResult {
                        url,
                        reachable: true,
                        latency_ms: Some(tcp_started_at.elapsed().as_millis() as u64),
                        error: None,
                    },
                    Ok(Err(tcp_error)) => DnsLatencyResult {
                        url,
                        reachable: false,
                        latency_ms: None,
                        error: Some(format!(
                            "ICMP не удался: {icmp_error}; fallback TCP не удался: {tcp_error}"
                        )),
                    },
                    Err(_) => DnsLatencyResult {
                        url,
                        reachable: false,
                        latency_ms: None,
                        error: Some(format!(
                            "ICMP не удался: {icmp_error}; fallback TCP превысил таймаут"
                        )),
                    },
                };
            }
            Err(_) => {
                let tcp_started_at = Instant::now();
                let tcp_result = timeout(
                    Duration::from_millis(1500),
                    TcpStream::connect(format!("{host}:{port}")),
                )
                .await;

                return match tcp_result {
                    Ok(Ok(_)) => DnsLatencyResult {
                        url,
                        reachable: true,
                        latency_ms: Some(tcp_started_at.elapsed().as_millis() as u64),
                        error: None,
                    },
                    Ok(Err(tcp_error)) => DnsLatencyResult {
                        url,
                        reachable: false,
                        latency_ms: None,
                        error: Some(format!(
                            "ICMP превысил таймаут; fallback TCP не удался: {tcp_error}"
                        )),
                    },
                    Err(_) => DnsLatencyResult {
                        url,
                        reachable: false,
                        latency_ms: None,
                        error: Some("ICMP и fallback TCP превысили таймаут".to_string()),
                    },
                };
            }
        }
    }

    let tcp_started_at = Instant::now();
    let tcp_result = timeout(
        Duration::from_millis(1500),
        TcpStream::connect(format!("{host}:{port}")),
    )
    .await;

    match tcp_result {
        Ok(Ok(_)) => DnsLatencyResult {
            url,
            reachable: true,
            latency_ms: Some(tcp_started_at.elapsed().as_millis() as u64),
            error: None,
        },
        Ok(Err(error)) => DnsLatencyResult {
            url,
            reachable: false,
            latency_ms: None,
            error: Some(format!("Не удалось подключиться по TCP: {error}")),
        },
        Err(_) => DnsLatencyResult {
            url,
            reachable: false,
            latency_ms: None,
            error: Some("Не удалось разрешить адрес и fallback TCP превысил таймаут".to_string()),
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
    let previous_status = get_dns_proxy_status_inner()?;
    let was_app_managed = previous_status.running && previous_status.app_managed;

    let backup = if was_app_managed {
        if let Some(existing) = get_reusable_dns_backup()? {
            existing
        } else {
            capture_fresh_dns_backup()?
        }
    } else {
        remove_dns_backup();
        capture_fresh_dns_backup()?
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
        cleanup_failed_dns_start(was_app_managed);
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
            cleanup_failed_dns_start(was_app_managed);
        }
        if rollback_errors.is_empty() {
            return Err(error);
        }

        return Err(format!(
            "{error}. Дополнительно не удалось откатить изменения: {}",
            rollback_errors.join(" | ")
        ));
    }

    write_dns_managed_marker()?;

    std::thread::sleep(std::time::Duration::from_millis(350));
    get_dns_proxy_status_inner()
}

fn stop_dns_proxy_inner() -> Result<DnsProxyStatus, String> {
    let mut shutdown_errors = Vec::new();

    if let Some(backup) = read_dns_backup()? {
        match restore_system_dns_from_backup(&backup) {
            Ok(()) => {}
            Err(error) => push_rollback_error(
                &mut shutdown_errors,
                "restore_system_dns_from_backup",
                Err(error),
            ),
        }
    } else {
        push_rollback_error(
            &mut shutdown_errors,
            "reset_system_dns_for_active_adapters",
            reset_system_dns_for_active_adapters(),
        );
    }

    let config_path = dns_proxy_config_path();
    push_rollback_error(
        &mut shutdown_errors,
        "run_dns_proxy_action(stop)",
        run_dns_proxy_action(&config_path, "stop").map(|_| ()),
    );
    push_rollback_error(
        &mut shutdown_errors,
        "run_dns_proxy_action(uninstall)",
        run_dns_proxy_action(&config_path, "uninstall").map(|_| ()),
    );

    if !shutdown_errors.is_empty() {
        return Err(format!(
            "Не удалось полностью отключить DNS модуль: {}",
            shutdown_errors.join(" | ")
        ));
    }

    remove_dns_backup();
    remove_dns_managed_marker();

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
