use std::fs;
use std::path::Path;
use std::sync::Mutex;

use anyhow::{Context as _, Result, bail};

use super::discord::DiscordPresence;
use super::dns::DnsRuntime;
use super::{hidden_cmd, process_name_running, remove_if_exists};
use crate::domain::{AppConfig, ConfigRepository, build_winws_args, validate_port_spec};

pub struct RuntimeServices {
    repository: ConfigRepository,
    state: Mutex<Processes>,
    discord: Mutex<DiscordPresence>,
}

pub struct ConnectOutcome {
    pub pid: u32,
    pub module_errors: Vec<String>,
}

struct Processes {
    winws: Option<duct::Handle>,
    tg_proxy: Option<duct::Handle>,
    dns: DnsRuntime,
}

impl RuntimeServices {
    pub fn new(repository: ConfigRepository) -> Result<Self> {
        let config_path = repository.config_path();
        let runtime_dir = config_path.parent().context("у config нет каталога")?;
        Ok(Self {
            state: Mutex::new(Processes {
                winws: None,
                tg_proxy: None,
                dns: DnsRuntime::new(runtime_dir),
            }),
            discord: Mutex::new(DiscordPresence::default()),
            repository,
        })
    }

    pub fn connect(&self, config: &AppConfig) -> Result<ConnectOutcome> {
        if !is_elevated() {
            bail!("для запуска WinDivert требуются права администратора");
        }
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("runtime lock poisoned"))?;
        state.stop_all()?;

        let child = spawn_winws(self.repository.resources_dir(), config)?;
        let pid = child
            .pids()
            .into_iter()
            .next()
            .context("winws запущен, но не вернул PID")?;
        if let Err(error) = fs::write(self.pid_path("winws.pid")?, pid.to_string()) {
            let mut child = Some(child);
            let stop_error = stop_child(&mut child, "winws").err();
            return match stop_error {
                Some(stop) => Err(anyhow::anyhow!(
                    "не удалось записать winws.pid: {error}; остановка winws также не удалась: {stop:#}"
                )),
                None => Err(error).context("не удалось записать winws.pid"),
            };
        }
        state.winws = Some(child);

        let mut module_errors = Vec::new();
        if config.dns_module_enabled
            && let Err(error) = state.dns.start(
                self.repository.resources_dir(),
                &config.dns_preset_id,
                config.dns_accelerator_enabled,
                &config.dns_bootstrap_resolvers,
            )
        {
            module_errors.push(format!("DNS: {error:#}"));
        }
        if config.tg_ws_proxy_module_enabled
            && let Err(error) = start_tg_proxy(
                &mut state.tg_proxy,
                self.repository.resources_dir(),
                config,
                &self.runtime_dir()?,
            )
        {
            module_errors.push(format!("Telegram WS Proxy: {error:#}"));
        }

        Ok(ConnectOutcome { pid, module_errors })
    }

    pub fn disconnect(&self) -> Result<()> {
        let stop_result = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("runtime lock poisoned"))?
            .stop_all();
        let pid_result = remove_if_exists(&self.pid_path("winws.pid")?);
        let tg_pid_result = remove_if_exists(&self.pid_path("tg-ws-proxy.pid")?);
        let driver_result = cleanup_driver_services();
        combine_results([stop_result, pid_result, tg_pid_result, driver_result])
    }

    pub fn sync_dns(&self, config: &AppConfig) -> Result<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("runtime lock poisoned"))?;
        if config.dns_module_enabled {
            state.dns.start(
                self.repository.resources_dir(),
                &config.dns_preset_id,
                config.dns_accelerator_enabled,
                &config.dns_bootstrap_resolvers,
            )
        } else {
            state.dns.stop()
        }
    }

    pub fn sync_tg_proxy(&self, config: &AppConfig) -> Result<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("runtime lock poisoned"))?;
        stop_child(&mut state.tg_proxy, "tg-ws-proxy")?;
        remove_if_exists(&self.pid_path("tg-ws-proxy.pid")?)?;
        if config.tg_ws_proxy_module_enabled {
            start_tg_proxy(
                &mut state.tg_proxy,
                self.repository.resources_dir(),
                config,
                &self.runtime_dir()?,
            )?;
        }
        Ok(())
    }

    fn runtime_dir(&self) -> Result<std::path::PathBuf> {
        self.repository
            .config_path()
            .parent()
            .map(Path::to_path_buf)
            .context("у config нет каталога")
    }

    fn pid_path(&self, name: &str) -> Result<std::path::PathBuf> {
        Ok(self.runtime_dir()?.join(name))
    }

    pub fn open_filters_directory(&self) -> Result<()> {
        open_directory(&self.repository.filters_dir())
    }

    pub fn open_placeholders_directory(&self) -> Result<()> {
        open_directory(self.repository.resources_dir())
    }

    #[cfg(windows)]
    pub fn open_external(&self, url: &str) -> Result<()> {
        hidden_cmd("rundll32.exe", ["url.dll,FileProtocolHandler", url])
            .start()
            .with_context(|| format!("не удалось открыть {url}"))?;
        Ok(())
    }

    #[cfg(not(windows))]
    pub fn open_external(&self, url: &str) -> Result<()> {
        duct::cmd("xdg-open", [url])
            .start()
            .with_context(|| format!("не удалось открыть {url}"))?;
        Ok(())
    }

    pub fn is_autostart_enabled(&self) -> bool {
        hidden_cmd(
            "reg.exe",
            [
                "query",
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                "/v",
                "ZapretInteractive",
            ],
        )
        .unchecked()
        .run()
        .is_ok_and(|out| out.status.success())
    }

    pub fn set_autostart_enabled(&self, enabled: bool) -> Result<()> {
        if !enabled && !self.is_autostart_enabled() {
            return Ok(());
        }
        if enabled {
            let executable =
                std::env::current_exe().context("не удалось определить ZapretInteractive.exe")?;
            hidden_cmd(
                "reg.exe",
                [
                    "add",
                    r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                    "/v",
                    "ZapretInteractive",
                    "/t",
                    "REG_SZ",
                    "/d",
                    &format!("\"{}\" --autostart", executable.display()),
                    "/f",
                ],
            )
            .run()
            .context("не удалось настроить автозапуск")?;
        } else {
            hidden_cmd(
                "reg.exe",
                [
                    "delete",
                    r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                    "/v",
                    "ZapretInteractive",
                    "/f",
                ],
            )
            .run()
            .context("не удалось отключить автозапуск")?;
        }
        Ok(())
    }

    pub fn sync_discord(&self, config: &AppConfig, connected: bool) -> Result<()> {
        self.discord
            .lock()
            .map_err(|_| anyhow::anyhow!("discord lock poisoned"))?
            .sync(config, connected)
    }

    pub fn tg_proxy_pid(&self) -> Option<u32> {
        self.state.lock().ok().and_then(|state| {
            state
                .tg_proxy
                .as_ref()
                .and_then(|handle| handle.pids().into_iter().next())
        })
    }

    pub fn initialize_system(&self) -> Result<&'static str> {
        if !is_elevated() {
            return Ok("Приложение запущено без прав администратора");
        }
        if enable_tcp_timestamps()? {
            Ok("TCP timestamps включены")
        } else {
            Ok("TCP timestamps уже включены")
        }
    }
}

impl Drop for RuntimeServices {
    fn drop(&mut self) {
        match self.state.lock() {
            Ok(mut state) => {
                if let Err(error) = state.stop_all() {
                    eprintln!("не удалось остановить внешние модули: {error:#}");
                }
            }
            Err(_) => eprintln!("не удалось остановить внешние модули: runtime lock poisoned"),
        }
        if let Err(error) = cleanup_driver_services() {
            eprintln!("не удалось очистить драйверные службы: {error:#}");
        }
    }
}

impl Processes {
    fn stop_all(&mut self) -> Result<()> {
        combine_results([
            stop_child(&mut self.winws, "winws"),
            stop_child(&mut self.tg_proxy, "tg-ws-proxy"),
            self.dns.stop(),
        ])
    }
}

fn start_tg_proxy(
    slot: &mut Option<duct::Handle>,
    resources: &Path,
    config: &AppConfig,
    runtime: &Path,
) -> Result<()> {
    let child = spawn_tg_proxy(resources, config, runtime)?;
    let pid = child
        .pids()
        .into_iter()
        .next()
        .context("tg-ws-proxy запущен, но не вернул PID")?;
    if let Err(error) = fs::write(runtime.join("tg-ws-proxy.pid"), pid.to_string()) {
        let mut child = Some(child);
        let stop_error = stop_child(&mut child, "tg-ws-proxy").err();
        return match stop_error {
            Some(stop) => Err(anyhow::anyhow!(
                "не удалось записать tg-ws-proxy.pid: {error}; остановка модуля также не удалась: {stop:#}"
            )),
            None => Err(error).context("не удалось записать tg-ws-proxy.pid"),
        };
    }
    *slot = Some(child);
    Ok(())
}

fn spawn_winws(resources: &Path, config: &AppConfig) -> Result<duct::Handle> {
    validate_port_spec(&config.global_ports.tcp).context("некорректные TCP-порты")?;
    validate_port_spec(&config.global_ports.udp).context("некорректные UDP-порты")?;
    let binary = resources.join("winws.exe");
    if !binary.is_file() {
        bail!("winws.exe не найден: {}", binary.display());
    }
    hidden_cmd(&binary, build_winws_args(config, resources))
        .dir(resources)
        .stdout_null()
        .stderr_null()
        .unchecked()
        .start()
        .with_context(|| format!("не удалось запустить {}", binary.display()))
}

fn spawn_tg_proxy(resources: &Path, config: &AppConfig, runtime: &Path) -> Result<duct::Handle> {
    if process_name_running("tg-ws-proxy.exe")? {
        bail!("tg-ws-proxy уже запущен вне управления приложения");
    }
    let secret = config.tg_ws_proxy_secret.trim().to_ascii_lowercase();
    if secret.len() != 32 || !secret.chars().all(|value| value.is_ascii_hexdigit()) {
        bail!("секрет TG-прокси должен содержать 32 hex-символа");
    }
    let binary = resources.join("modules/tg-ws-proxy-rs/tg-ws-proxy.exe");
    if !binary.is_file() {
        bail!("tg-ws-proxy.exe не найден: {}", binary.display());
    }
    let module_runtime = runtime.join("tg-ws-proxy-rs");
    fs::create_dir_all(&module_runtime).context("не удалось создать runtime TG-прокси")?;
    let binary_dir = binary.parent().context("у TG-модуля нет каталога")?;
    hidden_cmd(
        &binary,
        [
            "--host",
            "127.0.0.1",
            "--link-ip",
            "127.0.0.1",
            "--port",
            &config.tg_ws_proxy_port.to_string(),
            "--secret",
            &secret,
            "--quiet",
            "--log-file",
            &module_runtime.join("tg-ws-proxy.log").to_string_lossy(),
        ],
    )
    .dir(binary_dir)
    .stdout_null()
    .stderr_null()
    .unchecked()
    .start()
    .context("не удалось запустить TG-прокси")
}

fn stop_child(child: &mut Option<duct::Handle>, name: &str) -> Result<()> {
    if let Some(running) = child.take()
        && running
            .try_wait()
            .with_context(|| format!("не удалось проверить {name}"))?
            .is_none()
    {
        if let Some(pid) = running.pids().into_iter().next() {
            hidden_cmd("taskkill.exe", ["/F", "/PID", &pid.to_string(), "/T"])
                .run()
                .with_context(|| format!("не удалось остановить {name}, PID {pid}"))?;
        } else {
            running
                .kill()
                .with_context(|| format!("не удалось остановить {name}"))?;
        }
    }
    Ok(())
}

fn combine_results(results: impl IntoIterator<Item = Result<()>>) -> Result<()> {
    let errors = results
        .into_iter()
        .filter_map(Result::err)
        .map(|error| format!("{error:#}"))
        .collect::<Vec<_>>();
    if errors.is_empty() {
        Ok(())
    } else {
        bail!(errors.join("; "))
    }
}

fn cleanup_driver_services() -> Result<()> {
    let mut results = Vec::new();
    for name in ["WinDivert", "WinDivert14", "Monkey", "Monkey64"] {
        results.push(
            hidden_cmd("sc.exe", ["stop", name])
                .unchecked()
                .run()
                .map(|_| ())
                .with_context(|| format!("не удалось выполнить sc stop {name}")),
        );
        results.push(
            hidden_cmd("sc.exe", ["delete", name])
                .unchecked()
                .run()
                .map(|_| ())
                .with_context(|| format!("не удалось выполнить sc delete {name}")),
        );
    }
    combine_results(results)
}

fn open_directory(path: &Path) -> Result<()> {
    duct::cmd("explorer.exe", [path])
        .start()
        .with_context(|| format!("не удалось открыть {}", path.display()))?;
    Ok(())
}

pub fn cleanup_orphaned_processes(repository: &ConfigRepository) -> Result<()> {
    let config_path = repository.config_path();
    let runtime = config_path.parent().context("у config нет каталога")?;
    let managed = [
        ("winws.pid", repository.resources_dir().join("winws.exe")),
        (
            "dnscrypt-proxy/dnscrypt-proxy.pid",
            repository
                .resources_dir()
                .join("modules/dnscrypt-proxy/dnscrypt-proxy.exe"),
        ),
        (
            "tg-ws-proxy.pid",
            repository
                .resources_dir()
                .join("modules/tg-ws-proxy-rs/tg-ws-proxy.exe"),
        ),
    ];
    let mut results = Vec::with_capacity(managed.len() + 2);
    for (pid_file, executable) in managed {
        results.push(cleanup_managed_pid(&runtime.join(pid_file), &executable));
    }
    results.push(DnsRuntime::new(runtime).stop());
    results.push(cleanup_driver_services());
    combine_results(results)
}

fn cleanup_managed_pid(pid_file: &Path, expected_executable: &Path) -> Result<()> {
    let pid_text = match fs::read_to_string(pid_file) {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("не удалось прочитать {}", pid_file.display()));
        }
    };
    let pid = pid_text
        .trim()
        .parse::<u32>()
        .with_context(|| format!("некорректный PID в {}", pid_file.display()))?;
    if process_path_matches(pid, expected_executable)? {
        hidden_cmd("taskkill.exe", ["/F", "/PID", &pid.to_string(), "/T"])
            .run()
            .with_context(|| format!("не удалось остановить управляемый PID {pid}"))?;
    }
    remove_if_exists(pid_file)
}

fn process_path_matches(pid: u32, expected: &Path) -> Result<bool> {
    let output = hidden_cmd(
        "powershell.exe",
        [
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "(Get-CimInstance Win32_Process -Filter \"ProcessId = $env:ZK_PID\").ExecutablePath",
        ],
    )
    .env("ZK_PID", pid.to_string())
    .unchecked()
    .read()
    .with_context(|| format!("не удалось проверить путь процесса PID {pid}"))?;
    let actual = output.trim();
    if actual.is_empty() {
        return Ok(false);
    }
    let expected = fs::canonicalize(expected).unwrap_or_else(|_| expected.to_path_buf());
    Ok(actual.eq_ignore_ascii_case(&expected.to_string_lossy()))
}

pub fn is_elevated() -> bool {
    hidden_cmd("fltmc.exe", [] as [&str; 0])
        .stdout_null()
        .stderr_null()
        .unchecked()
        .run()
        .is_ok_and(|out| out.status.success())
}

fn enable_tcp_timestamps() -> Result<bool> {
    let current = read_tcp_timestamp_value()?;
    if current & 0b10 != 0 {
        return Ok(false);
    }
    let next = current | 0b10;
    hidden_cmd(
        "reg.exe",
        [
            "add",
            r"HKLM\SYSTEM\CurrentControlSet\Services\Tcpip\Parameters",
            "/v",
            "Tcp1323Opts",
            "/t",
            "REG_DWORD",
            "/d",
            &next.to_string(),
            "/f",
        ],
    )
    .run()
    .context("не удалось включить TCP timestamps")?;
    Ok(true)
}

fn read_tcp_timestamp_value() -> Result<u32> {
    let output = hidden_cmd(
        "reg.exe",
        [
            "query",
            r"HKLM\SYSTEM\CurrentControlSet\Services\Tcpip\Parameters",
            "/v",
            "Tcp1323Opts",
        ],
    )
    .unchecked()
    .read()
    .context("не удалось проверить TCP timestamps")?;
    parse_registry_dword(&output).context("reg.exe вернул некорректный Tcp1323Opts")
}

fn parse_registry_dword(output: &str) -> Option<u32> {
    output
        .split_whitespace()
        .find_map(|value| value.strip_prefix("0x"))
        .and_then(|value| u32::from_str_radix(value, 16).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_registry_dword() {
        assert_eq!(
            parse_registry_dword("Tcp1323Opts    REG_DWORD    0x3"),
            Some(3)
        );
        assert_eq!(
            parse_registry_dword("Tcp1323Opts    REG_DWORD    0x0"),
            Some(0)
        );
        assert_eq!(
            parse_registry_dword("Setting    REG_DWORD    0x10"),
            Some(16)
        );
        assert_eq!(
            parse_registry_dword("Setting    REG_DWORD    0xff"),
            Some(255)
        );
        assert_eq!(
            parse_registry_dword("Invalid    REG_SZ    some_string"),
            None
        );
        assert_eq!(parse_registry_dword(""), None);
    }
}
