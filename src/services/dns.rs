use std::collections::HashMap;
use std::fs;
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result, bail};
use dnsstamps::DoHBuilder;
use toml::{Table, Value};
use url::Url;

use super::{hidden_cmd, process_name_running, remove_if_exists};

pub const PRESETS: [(&str, &str, &str); 6] = [
    ("comss-one", "Comss", "https://dns.comss.one/dns-query"),
    ("xbox-dns-ru", "Xbox DNS", "https://xbox-dns.ru/dns-query"),
    (
        "malw-link-main",
        "Malw Link",
        "https://dns.malw.link/dns-query",
    ),
    (
        "malw-link-cf",
        "Malw Link (Cloudflare)",
        "https://5u35p8m9i7.cloudflare-gateway.com/dns-query",
    ),
    (
        "mafioznik",
        "Mafioznik",
        "https://dns.mafioznik.xyz/dns-query",
    ),
    ("astracat", "Astracat", "https://dns.astracat.ru/dns-query"),
];

pub fn measure_preset_latencies() -> Result<HashMap<String, Option<u128>>> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("не удалось создать runtime проверки DNS")?;
    runtime.block_on(async {
        let mut tasks = tokio::task::JoinSet::new();
        for (id, _, endpoint) in PRESETS {
            tasks.spawn(async move { (id.to_owned(), measure_dns_latency(endpoint).await) });
        }
        let mut results = HashMap::with_capacity(PRESETS.len());
        while let Some(result) = tasks.join_next().await {
            let (id, latency) = result.context("задача проверки DNS завершилась аварийно")?;
            results.insert(id, latency);
        }
        Ok(results)
    })
}

async fn measure_dns_latency(endpoint: &str) -> Option<u128> {
    let url = Url::parse(endpoint).ok()?;
    let host = url.host_str()?.to_owned();
    let port = url.port_or_known_default().unwrap_or(443);
    let ip = tokio::net::lookup_host((host.as_str(), port))
        .await
        .ok()?
        .next()?
        .ip();
    let payload = [0_u8; 8];
    if let Ok(Ok((_, latency))) =
        tokio::time::timeout(Duration::from_millis(1_500), surge_ping::ping(ip, &payload)).await
    {
        return Some(latency.as_millis());
    }
    let started = Instant::now();
    tokio::time::timeout(
        Duration::from_millis(1_500),
        tokio::net::TcpStream::connect((host.as_str(), port)),
    )
    .await
    .ok()?
    .ok()?;
    Some(started.elapsed().as_millis())
}

pub struct DnsRuntime {
    child: Option<duct::Handle>,
    runtime_dir: PathBuf,
}

impl DnsRuntime {
    pub fn new(runtime_dir: &Path) -> Self {
        Self {
            child: None,
            runtime_dir: runtime_dir.join("dnscrypt-proxy"),
        }
    }

    pub fn start(
        &mut self,
        resources_dir: &Path,
        preset_id: &str,
        accelerator: bool,
        bootstrap: &[String],
    ) -> Result<()> {
        self.stop()?;
        if dnscrypt_is_running()? {
            bail!("dnscrypt-proxy уже запущен вне управления приложения");
        }
        fs::create_dir_all(&self.runtime_dir).context("не удалось создать runtime DNS")?;
        let endpoint = dns_endpoint(preset_id, accelerator);
        let config = write_config(&self.runtime_dir, &endpoint, bootstrap)?;
        let binary = resources_dir.join("modules/dnscrypt-proxy/dnscrypt-proxy.exe");
        if !binary.is_file() {
            bail!("dnscrypt-proxy.exe не найден: {}", binary.display());
        }
        let binary_dir = binary.parent().context("у DNS-модуля нет каталога")?;
        validate_config(&binary, binary_dir, &config)?;
        let child = hidden_cmd(&binary, ["-config", &config.to_string_lossy()])
            .dir(binary_dir)
            .stdout_null()
            .stderr_null()
            .start()
            .with_context(|| format!("не удалось запустить {}", binary.display()));
        match child {
            Ok(child) => {
                let pid = match child.pids().into_iter().next() {
                    Some(pid) => pid,
                    None => {
                        let _intentionally_ignored = child.kill();
                        restore_dns(&self.runtime_dir)?;
                        bail!("dnscrypt-proxy запущен, но не вернул PID");
                    }
                };
                if let Err(error) =
                    fs::write(self.runtime_dir.join("dnscrypt-proxy.pid"), pid.to_string())
                {
                    let stop_error = child.kill().err();
                    bail!(
                        "не удалось записать dnscrypt-proxy.pid: {error}; остановка: {}",
                        format_optional_error(stop_error)
                    );
                }
                if let Err(error) = wait_for_dns_proxy(Duration::from_secs(5)) {
                    let stop_error = child.kill().err();
                    let _intentionally_ignored =
                        remove_if_exists(&self.runtime_dir.join("dnscrypt-proxy.pid"));
                    bail!(
                        "{error:#}; остановка: {}",
                        format_optional_error(stop_error)
                    );
                }
                if let Err(error) = backup_and_apply_dns(&self.runtime_dir) {
                    let stop_error = child.kill().err();
                    let restore_error = restore_dns(&self.runtime_dir).err();
                    let _intentionally_ignored =
                        remove_if_exists(&self.runtime_dir.join("dnscrypt-proxy.pid"));
                    bail!(
                        "{error:#}; остановка: {}; откат DNS: {}",
                        format_optional_error(stop_error),
                        format_optional_error(restore_error)
                    );
                }
                self.child = Some(child);
                Ok(())
            }
            Err(error) => Err(error),
        }
    }

    pub fn stop(&mut self) -> Result<()> {
        let mut errors = Vec::new();
        if let Some(child) = self.child.take()
            && let Err(error) = child.kill().context("не удалось остановить dnscrypt-proxy")
        {
            errors.push(format!("{error:#}"));
        }
        if let Err(error) = remove_if_exists(&self.runtime_dir.join("dnscrypt-proxy.pid")) {
            errors.push(format!("{error:#}"));
        }
        if let Err(error) = restore_dns(&self.runtime_dir) {
            errors.push(format!("{error:#}"));
        }
        if errors.is_empty() {
            Ok(())
        } else {
            bail!(errors.join("; "))
        }
    }
}

fn dnscrypt_is_running() -> Result<bool> {
    process_name_running("dnscrypt-proxy.exe")
}

fn format_optional_error(error: Option<impl std::fmt::Display>) -> String {
    error.map_or_else(|| "успешно".to_owned(), |error| format!("{error:#}"))
}

fn dns_endpoint(preset_id: &str, multiqueue: bool) -> String {
    let selected = PRESETS
        .iter()
        .find(|preset| preset.0 == preset_id)
        .unwrap_or(&PRESETS[0])
        .2;
    if !multiqueue {
        return selected.to_owned();
    }
    let upstreams = PRESETS
        .iter()
        .map(|preset| {
            preset
                .2
                .split_once("://")
                .map_or(preset.2, |(_, upstream)| upstream)
        })
        .collect::<Vec<_>>()
        .join("/mq/");
    format!("https://v.recipes/mq/{upstreams}")
}

fn write_config(runtime_dir: &Path, endpoint: &str, bootstrap: &[String]) -> Result<PathBuf> {
    let mut root = Table::new();
    let mut bootstrap_array = vec![
        Value::String("9.9.9.9:53".into()),
        Value::String("1.1.1.1:53".into()),
    ];
    for resolver in bootstrap {
        let trimmed = resolver.trim();
        if !trimmed.is_empty() {
            bootstrap_array.push(Value::String(normalize_bootstrap_resolver(trimmed)?));
        }
    }
    root.insert(
        "listen_addresses".into(),
        Value::Array(vec![
            Value::String("127.0.0.1:53".into()),
            Value::String("[::1]:53".into()),
        ]),
    );
    root.insert(
        "server_names".into(),
        Value::Array(vec![Value::String("static-preset".into())]),
    );
    root.insert("bootstrap_resolvers".into(), Value::Array(bootstrap_array));
    root.insert("ignore_system_dns".into(), Value::Boolean(true));
    root.insert(
        "netprobe_address".into(),
        Value::String("9.9.9.9:53".into()),
    );
    root.insert("block_ipv6".into(), Value::Boolean(true));
    root.insert("cache".into(), Value::Boolean(true));
    root.insert("cache_size".into(), Value::Integer(4096));
    root.insert("cache_min_ttl".into(), Value::Integer(300));
    root.insert("cache_max_ttl".into(), Value::Integer(86400));
    root.insert("cache_neg_min_ttl".into(), Value::Integer(60));
    root.insert("cache_neg_max_ttl".into(), Value::Integer(600));

    let mut static_table = Table::new();
    let mut preset_table = Table::new();
    let url = Url::parse(endpoint).with_context(|| format!("некорректный DoH URL: {endpoint}"))?;
    let host = url.host_str().context("DoH URL не содержит хост")?;
    let stamp = DoHBuilder::new(host.to_owned(), url.path().to_owned())
        .with_port(url.port_or_known_default().unwrap_or(443))
        .serialize()
        .context("не удалось собрать DNS stamp")?;
    preset_table.insert("stamp".into(), Value::String(stamp));
    static_table.insert("static-preset".into(), Value::Table(preset_table));
    root.insert("static".into(), Value::Table(static_table));
    let path = runtime_dir.join("dnscrypt-proxy.toml");
    fs::write(
        &path,
        toml::to_string_pretty(&root).context("не удалось сериализовать DNS config")?,
    )
    .context("не удалось записать DNS config")?;
    Ok(path)
}

fn normalize_bootstrap_resolver(resolver: &str) -> Result<String> {
    if let Ok(address) = resolver.parse::<SocketAddr>() {
        return Ok(address.to_string());
    }
    let address = resolver
        .parse::<IpAddr>()
        .with_context(|| format!("некорректный bootstrap DNS: {resolver}"))?;
    Ok(SocketAddr::new(address, 53).to_string())
}

fn validate_config(binary: &Path, binary_dir: &Path, config: &Path) -> Result<()> {
    let output = hidden_cmd(binary, ["-check", "-config", &config.to_string_lossy()])
        .dir(binary_dir)
        .stdout_capture()
        .stderr_capture()
        .unchecked()
        .run()
        .context("не удалось проверить конфигурацию dnscrypt-proxy")?;
    if output.status.success() {
        return Ok(());
    }
    let message = String::from_utf8_lossy(&output.stderr);
    bail!("dnscrypt-proxy отклонил конфигурацию: {}", message.trim())
}

fn wait_for_dns_proxy(timeout: Duration) -> Result<()> {
    let address = SocketAddr::from(([127, 0, 0, 1], 53));
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if TcpStream::connect_timeout(&address, Duration::from_millis(100)).is_ok() {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(50));
    }
    bail!("dnscrypt-proxy не открыл локальный DNS-порт за отведённое время")
}

fn powershell(script: &str, runtime_dir: &Path) -> Result<()> {
    hidden_cmd(
        "powershell.exe",
        ["-NoProfile", "-NonInteractive", "-Command", script],
    )
    .env("ZK_DNS_DIR", runtime_dir)
    .run()
    .context("не удалось запустить PowerShell для настройки DNS")?;
    Ok(())
}

fn backup_and_apply_dns(runtime_dir: &Path) -> Result<()> {
    powershell(
        "$p=Join-Path $env:ZK_DNS_DIR 'dns-backup.json'; $a=Get-NetAdapter | Where-Object Status -eq 'Up'; $b=@($a | ForEach-Object { $i=$_.ifIndex; [pscustomobject]@{Index=$i;V4=@((Get-DnsClientServerAddress -InterfaceIndex $i -AddressFamily IPv4).ServerAddresses);V6=@((Get-DnsClientServerAddress -InterfaceIndex $i -AddressFamily IPv6).ServerAddresses)} }); $b | ConvertTo-Json -Depth 4 | Set-Content -Encoding UTF8 $p; $a | ForEach-Object { Set-DnsClientServerAddress -InterfaceIndex $_.ifIndex -ServerAddresses @('127.0.0.1','::1') -ErrorAction Stop }",
        runtime_dir,
    )
}

fn restore_dns(runtime_dir: &Path) -> Result<()> {
    if !runtime_dir.join("dns-backup.json").is_file() {
        return Ok(());
    }
    powershell(
        "$p=Join-Path $env:ZK_DNS_DIR 'dns-backup.json'; $b=@(Get-Content -Raw $p | ConvertFrom-Json); foreach($x in $b){$s=@($x.V4)+@($x.V6); if($s.Count -eq 0){Set-DnsClientServerAddress -InterfaceIndex $x.Index -ResetServerAddresses -ErrorAction Stop}else{Set-DnsClientServerAddress -InterfaceIndex $x.Index -ServerAddresses $s -ErrorAction Stop}}; Remove-Item -LiteralPath $p -Force",
        runtime_dir,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multiqueue_uses_every_preset() {
        let endpoint = dns_endpoint(PRESETS[0].0, true);
        assert_eq!(endpoint.matches("/mq/").count(), PRESETS.len());
        for (_, _, upstream) in PRESETS {
            assert!(endpoint.contains(upstream.trim_start_matches("https://")));
        }
    }

    #[test]
    fn single_mode_uses_selected_preset() {
        assert_eq!(
            dns_endpoint("xbox-dns-ru", false),
            "https://xbox-dns.ru/dns-query"
        );
    }

    #[test]
    fn bootstrap_resolvers_get_required_ports() {
        assert_eq!(
            normalize_bootstrap_resolver("77.88.8.8").unwrap(),
            "77.88.8.8:53"
        );
        assert_eq!(
            normalize_bootstrap_resolver("1.1.1.1:5353").unwrap(),
            "1.1.1.1:5353"
        );
        assert!(normalize_bootstrap_resolver("dns.example.com").is_err());
    }

    #[cfg(windows)]
    #[test]
    fn generated_default_config_is_accepted_by_dnscrypt_proxy() {
        let runtime_dir =
            std::env::temp_dir().join(format!("zapret-dns-config-test-{}", std::process::id()));
        fs::create_dir_all(&runtime_dir).unwrap();
        let config = write_config(
            &runtime_dir,
            PRESETS[0].2,
            &["77.88.8.8".into(), "1.1.1.1".into(), "8.8.8.8".into()],
        )
        .unwrap();
        let binary_dir =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("thirdparty/modules/dnscrypt-proxy");
        let result = validate_config(&binary_dir.join("dnscrypt-proxy.exe"), &binary_dir, &config);
        let _intentionally_ignored = fs::remove_dir_all(&runtime_dir);
        assert!(result.is_ok(), "{result:?}");
    }

    #[test]
    fn test_dns_presets_are_valid() {
        assert!(!PRESETS.is_empty());
        for (id, name, doh_url) in PRESETS {
            assert!(!id.is_empty());
            assert!(!name.is_empty());
            assert!(!doh_url.is_empty());
            assert!(doh_url.starts_with("https://") || doh_url.starts_with("http://"));
        }
    }
}
