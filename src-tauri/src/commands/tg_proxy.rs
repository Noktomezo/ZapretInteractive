use super::config::{get_managed_resources_dir, get_runtime_data_dir};
use duct::{Expression, Handle, cmd};
use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use sysinfo::{Pid, ProcessesToUpdate, System};

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
use windows::Win32::Foundation::{
    CloseHandle, HANDLE, INVALID_HANDLE_VALUE, WAIT_FAILED, WAIT_OBJECT_0, WAIT_TIMEOUT,
};
#[cfg(windows)]
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
#[cfg(windows)]
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
    PROCESS_TERMINATE, QueryFullProcessImageNameW, TerminateProcess, WaitForSingleObject,
};
#[cfg(windows)]
use windows::core::PWSTR;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;
const TG_WS_PROXY_PROCESS_NAME: &str = "tg-ws-proxy.exe";
const TG_WS_PROXY_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

static TG_WS_PROXY_PID: AtomicU32 = AtomicU32::new(0);
static TG_WS_PROXY_HANDLE: Mutex<Option<Handle>> = Mutex::new(None);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TgWsProxyStatus {
    running: bool,
    module_available: bool,
    binary_path: String,
    log_path: String,
    pid: Option<u32>,
}

fn tg_ws_proxy_module_dir() -> std::path::PathBuf {
    get_managed_resources_dir()
        .join("modules")
        .join("tg-ws-proxy-rs")
}

fn tg_ws_proxy_binary_path() -> std::path::PathBuf {
    tg_ws_proxy_module_dir().join(TG_WS_PROXY_PROCESS_NAME)
}

fn tg_ws_proxy_runtime_dir() -> std::path::PathBuf {
    get_runtime_data_dir().join("tg-ws-proxy-rs")
}

fn tg_ws_proxy_log_path() -> std::path::PathBuf {
    tg_ws_proxy_runtime_dir().join("tg-ws-proxy.log")
}

fn tg_ws_proxy_pid_path() -> std::path::PathBuf {
    tg_ws_proxy_runtime_dir().join("tg-ws-proxy.pid")
}

fn ensure_tg_ws_proxy_runtime_dir() -> Result<std::path::PathBuf, String> {
    let dir = tg_ws_proxy_runtime_dir();
    std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    Ok(dir)
}

fn write_tg_ws_proxy_pid(pid: u32) -> Result<(), String> {
    ensure_tg_ws_proxy_runtime_dir()?;
    std::fs::write(tg_ws_proxy_pid_path(), pid.to_string()).map_err(|error| error.to_string())
}

fn read_tg_ws_proxy_pid() -> Option<u32> {
    let content = std::fs::read_to_string(tg_ws_proxy_pid_path()).ok()?;
    content.trim().parse::<u32>().ok().filter(|pid| *pid != 0)
}

fn clear_tg_ws_proxy_pid() -> Result<(), String> {
    match std::fs::remove_file(tg_ws_proxy_pid_path()) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}

fn validate_tg_ws_proxy_secret(secret: &str) -> Result<String, String> {
    let normalized = secret.trim().to_ascii_lowercase();
    if normalized.len() != 32 || !normalized.chars().all(|char| char.is_ascii_hexdigit()) {
        return Err("Секрет должен состоять из 32 шестнадцатеричных символов".to_string());
    }
    Ok(normalized)
}

#[cfg(windows)]
fn normalize_path_for_compare(path: &Path) -> String {
    std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .replace('/', "\\")
        .to_ascii_lowercase()
}

#[cfg(windows)]
fn query_process_image_path(handle: HANDLE) -> Option<PathBuf> {
    unsafe {
        let mut size = 32_768u32;
        let mut buffer = vec![0u16; size as usize];
        QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            &mut size,
        )
        .ok()?;
        Some(PathBuf::from(String::from_utf16_lossy(
            &buffer[..size as usize],
        )))
    }
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

#[cfg(windows)]
fn terminate_process_by_pid(pid: u32) -> Result<(), String> {
    let mut errors = Vec::new();

    unsafe {
        match OpenProcess(
            PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE,
            false,
            pid,
        ) {
            Ok(handle) => match TerminateProcess(handle, 1) {
                Ok(()) => {
                    let wait_result = WaitForSingleObject(handle, 5_000);
                    let _ = CloseHandle(handle);

                    if wait_result == WAIT_OBJECT_0 {
                        return Ok(());
                    } else if wait_result == WAIT_TIMEOUT {
                        errors.push(format!(
                            "Process {pid} did not exit within 5000 ms after WinAPI termination"
                        ));
                    } else if wait_result == WAIT_FAILED {
                        errors.push(format!(
                            "Failed to wait for process {pid} termination after WinAPI kill"
                        ));
                    } else {
                        errors.push(format!(
                            "Unexpected wait result while terminating process {pid}: {:?}",
                            wait_result
                        ));
                    }
                }
                Err(error) => {
                    let _ = CloseHandle(handle);
                    errors.push(format!("Failed to terminate process {pid}: {error}"));
                }
            },
            Err(error) => errors.push(format!("Failed to open process {pid}: {error}")),
        }
    }

    if kill_process_by_pid_sysinfo(pid) {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while std::time::Instant::now() < deadline {
            if !is_process_running_by_pid(pid) {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        errors.push(format!(
            "Process {pid} did not exit within 5000 ms after sysinfo kill"
        ));
    } else {
        errors.push(format!("sysinfo failed to kill process {pid}"));
    }

    match kill_process_by_pid_taskkill(pid) {
        Ok(()) => {
            let deadline = std::time::Instant::now() + Duration::from_secs(5);
            while std::time::Instant::now() < deadline {
                if !is_process_running_by_pid(pid) {
                    return Ok(());
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            errors.push(format!(
                "Process {pid} did not exit within 5000 ms after taskkill fallback"
            ));
        }
        Err(error) => errors.push(error),
    }

    Err(errors.join(" | "))
}

#[cfg(windows)]
fn is_expected_tg_ws_proxy_process(pid: u32) -> bool {
    unsafe {
        let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
            return false;
        };

        let wait_result = WaitForSingleObject(handle, 0);
        let expected_path = normalize_path_for_compare(&tg_ws_proxy_binary_path());
        let actual_path = query_process_image_path(handle)
            .map(|path| normalize_path_for_compare(&path))
            .is_some_and(|path| path == expected_path);
        let _ = CloseHandle(handle);

        wait_result == WAIT_TIMEOUT && actual_path
    }
}

#[cfg(windows)]
fn find_process_pids_by_name(process_name: &str) -> Vec<u32> {
    unsafe {
        let Ok(snapshot) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
            return Vec::new();
        };
        if snapshot == INVALID_HANDLE_VALUE {
            return Vec::new();
        }

        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };

        let mut pids = Vec::new();
        if Process32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                let len = entry
                    .szExeFile
                    .iter()
                    .position(|&char| char == 0)
                    .unwrap_or(entry.szExeFile.len());
                let exe = String::from_utf16_lossy(&entry.szExeFile[..len]);
                if exe.eq_ignore_ascii_case(process_name) {
                    pids.push(entry.th32ProcessID);
                }

                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }

        let _ = CloseHandle(snapshot);
        pids
    }
}

#[cfg(windows)]
fn find_any_tg_ws_proxy_pids() -> Vec<u32> {
    find_process_pids_by_name(TG_WS_PROXY_PROCESS_NAME)
}

#[cfg(not(windows))]
fn recover_running_pids() -> Vec<u32> {
    Vec::new()
}

#[cfg(windows)]
fn recover_running_pids() -> Vec<u32> {
    find_process_pids_by_name(TG_WS_PROXY_PROCESS_NAME)
        .into_iter()
        .filter(|pid| is_expected_tg_ws_proxy_process(*pid))
        .collect()
}

#[cfg(windows)]
fn is_process_running_by_pid(pid: u32) -> bool {
    is_expected_tg_ws_proxy_process(pid)
}

fn clear_stored_handle() {
    if let Ok(mut handle) = TG_WS_PROXY_HANDLE.lock() {
        *handle = None;
    }
}

fn kill_process_by_pid_sysinfo(pid: u32) -> bool {
    let target_pid = Pid::from_u32(pid);
    let mut system = System::new_all();
    system.refresh_processes(ProcessesToUpdate::Some(&[target_pid]), true);
    system
        .process(target_pid)
        .is_some_and(|process| process.kill())
}

#[cfg(windows)]
fn kill_process_by_pid_taskkill(pid: u32) -> Result<(), String> {
    let output = std::process::Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .output()
        .map_err(|error| format!("Failed to launch taskkill for process {pid}: {error}"))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            format!("exit code {}", output.status)
        };
        Err(format!("taskkill failed for process {pid}: {detail}"))
    }
}

#[cfg(windows)]
fn collect_tg_ws_proxy_pid_candidates() -> Vec<u32> {
    let mut seen = HashSet::new();
    let mut pids = Vec::new();

    let stored_pid = TG_WS_PROXY_PID.load(Ordering::SeqCst);
    if stored_pid != 0 && seen.insert(stored_pid) {
        pids.push(stored_pid);
    }

    if let Some(pid) = read_tg_ws_proxy_pid()
        && seen.insert(pid)
    {
        pids.push(pid);
    }

    for pid in recover_running_pids() {
        if seen.insert(pid) {
            pids.push(pid);
        }
    }

    for pid in find_any_tg_ws_proxy_pids() {
        if seen.insert(pid) {
            pids.push(pid);
        }
    }

    pids
}

pub(crate) fn cleanup_orphaned_tg_ws_proxy_on_startup() -> Result<(), String> {
    let Some(pid) = read_tg_ws_proxy_pid() else {
        return Ok(());
    };

    if is_process_running_by_pid(pid) {
        terminate_process_by_pid(pid)?;
    }

    clear_stored_handle();
    TG_WS_PROXY_PID.store(0, Ordering::SeqCst);
    clear_tg_ws_proxy_pid()?;
    Ok(())
}

fn get_tg_ws_proxy_status_inner() -> Result<TgWsProxyStatus, String> {
    ensure_tg_ws_proxy_runtime_dir()?;

    let stored_pid = TG_WS_PROXY_PID.load(Ordering::SeqCst);
    #[cfg(windows)]
    let running_pid = if stored_pid != 0 && is_process_running_by_pid(stored_pid) {
        Some(stored_pid)
    } else {
        let recovered = read_tg_ws_proxy_pid()
            .filter(|pid| is_process_running_by_pid(*pid))
            .or_else(|| recover_running_pids().into_iter().next());
        if let Some(pid) = recovered {
            TG_WS_PROXY_PID.store(pid, Ordering::SeqCst);
            let _ = write_tg_ws_proxy_pid(pid);
        } else {
            TG_WS_PROXY_PID.store(0, Ordering::SeqCst);
            clear_stored_handle();
            let _ = clear_tg_ws_proxy_pid();
        }
        recovered
    };

    #[cfg(not(windows))]
    let running_pid = {
        let _ = stored_pid;
        None
    };

    Ok(TgWsProxyStatus {
        running: running_pid.is_some(),
        module_available: tg_ws_proxy_binary_path().is_file(),
        binary_path: tg_ws_proxy_binary_path().to_string_lossy().to_string(),
        log_path: tg_ws_proxy_log_path().to_string_lossy().to_string(),
        pid: running_pid,
    })
}

fn stop_tg_ws_proxy_inner() -> Result<TgWsProxyStatus, String> {
    let mut stop_errors = Vec::new();
    let handle = TG_WS_PROXY_HANDLE
        .lock()
        .map_err(|error| error.to_string())?
        .take();

    if let Some(handle) = handle {
        match handle.try_wait() {
            Ok(None) => {
                if let Err(error) = handle.kill() {
                    stop_errors.push(format!("Failed to kill tg-ws-proxy: {error}"));
                }
                let _ = handle.wait_timeout(TG_WS_PROXY_SHUTDOWN_TIMEOUT);
            }
            Ok(Some(_)) => {}
            Err(error) => {
                stop_errors.push(format!("Failed to inspect tg-ws-proxy state: {error}"));
            }
        }
    }

    #[cfg(windows)]
    {
        for pid in collect_tg_ws_proxy_pid_candidates() {
            if let Err(error) = terminate_process_by_pid(pid) {
                stop_errors.push(format!(
                    "Failed to terminate tg-ws-proxy process {pid}: {error}"
                ));
            }
        }
    }

    let shutdown_deadline = std::time::Instant::now() + TG_WS_PROXY_SHUTDOWN_TIMEOUT;
    let status = loop {
        let status = get_tg_ws_proxy_status_inner()?;
        if !status.running {
            break status;
        }

        if std::time::Instant::now() >= shutdown_deadline {
            stop_errors.push("tg-ws-proxy.exe всё ещё запущен после остановки".to_string());
            break status;
        }

        std::thread::sleep(Duration::from_millis(100));
    };

    if status.running {
        if let Some(pid) = status.pid {
            TG_WS_PROXY_PID.store(pid, Ordering::SeqCst);
            let _ = write_tg_ws_proxy_pid(pid);
        }
    } else {
        TG_WS_PROXY_PID.store(0, Ordering::SeqCst);
        let _ = clear_tg_ws_proxy_pid();
    }

    if stop_errors.is_empty() || !status.running {
        Ok(status)
    } else {
        Err(stop_errors.join(" | "))
    }
}

fn spawn_tg_ws_proxy_once(port: u16, secret: &str) -> Result<TgWsProxyStatus, String> {
    let binary_path = tg_ws_proxy_binary_path();
    if !binary_path.is_file() {
        return Err("tg-ws-proxy.exe не найден в resources/modules/tg-ws-proxy-rs".to_string());
    }

    let args = vec![
        "--host".to_string(),
        "127.0.0.1".to_string(),
        "--link-ip".to_string(),
        "127.0.0.1".to_string(),
        "--port".to_string(),
        port.to_string(),
        "--secret".to_string(),
        secret.to_string(),
        "--quiet".to_string(),
        "--log-file".to_string(),
        tg_ws_proxy_log_path().to_string_lossy().into_owned(),
    ];

    let handle = configure_expression(cmd(binary_path.to_string_lossy().into_owned(), args))
        .dir(tg_ws_proxy_module_dir())
        .start()
        .map_err(|error| format!("Failed to start tg-ws-proxy.exe: {error}"))?;

    let pid = handle
        .pids()
        .into_iter()
        .next()
        .ok_or_else(|| "Failed to get tg-ws-proxy PID from duct handle".to_string())?;

    {
        let mut running_handle = TG_WS_PROXY_HANDLE
            .lock()
            .map_err(|error| error.to_string())?;
        *running_handle = Some(handle);
    }
    TG_WS_PROXY_PID.store(pid, Ordering::SeqCst);
    write_tg_ws_proxy_pid(pid)?;

    let startup_deadline = std::time::Instant::now() + Duration::from_secs(2);
    loop {
        let exit_message = {
            let mut running_handle = TG_WS_PROXY_HANDLE
                .lock()
                .map_err(|error| error.to_string())?;
            match running_handle.take() {
                Some(handle) => {
                    let exit_message = handle
                        .try_wait()
                        .map_err(|error| format!("Failed to inspect tg-ws-proxy startup: {error}"))?
                        .map(|status| {
                            format!("tg-ws-proxy завершился сразу после запуска ({status:?})")
                        });
                    if exit_message.is_none() {
                        *running_handle = Some(handle);
                    }
                    exit_message
                }
                None => None,
            }
        };

        if let Some(message) = exit_message {
            TG_WS_PROXY_PID.store(0, Ordering::SeqCst);
            clear_stored_handle();
            let _ = clear_tg_ws_proxy_pid();
            return Err(message);
        }

        if std::time::Instant::now() >= startup_deadline {
            break;
        }

        std::thread::sleep(Duration::from_millis(100));
    }

    get_tg_ws_proxy_status_inner()
}

fn start_tg_ws_proxy_inner(port: u16, secret: String) -> Result<TgWsProxyStatus, String> {
    if port == 0 {
        return Err("Порт должен быть больше 0".to_string());
    }

    let secret = validate_tg_ws_proxy_secret(&secret)?;
    ensure_tg_ws_proxy_runtime_dir()?;
    if let Err(error) = stop_tg_ws_proxy_inner() {
        return Err(format!(
            "Failed to stop existing tg-ws-proxy before restart: {error}"
        ));
    }

    match spawn_tg_ws_proxy_once(port, &secret) {
        Ok(status) => Ok(status),
        Err(error) if error.contains("code 101") => {
            let _ = stop_tg_ws_proxy_inner();
            std::thread::sleep(Duration::from_millis(300));
            spawn_tg_ws_proxy_once(port, &secret).map_err(|retry_error| {
                format!("{error}. Повторный запуск не удался: {retry_error}")
            })
        }
        Err(error) => Err(error),
    }
}

#[tauri::command]
pub fn get_tg_ws_proxy_status() -> Result<TgWsProxyStatus, String> {
    get_tg_ws_proxy_status_inner()
}

#[tauri::command]
pub async fn start_tg_ws_proxy(port: u16, secret: String) -> Result<TgWsProxyStatus, String> {
    tauri::async_runtime::spawn_blocking(move || start_tg_ws_proxy_inner(port, secret))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn stop_tg_ws_proxy() -> Result<TgWsProxyStatus, String> {
    tauri::async_runtime::spawn_blocking(stop_tg_ws_proxy_inner)
        .await
        .map_err(|error| error.to_string())?
}
