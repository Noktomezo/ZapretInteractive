use super::config::{get_managed_resources_dir, get_runtime_data_dir};
use duct::{Expression, Handle, cmd};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
use windows::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE, WAIT_TIMEOUT};
#[cfg(windows)]
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
#[cfg(windows)]
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE,
    QueryFullProcessImageNameW, TerminateProcess, WaitForSingleObject,
};
#[cfg(windows)]
use windows::core::PWSTR;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;
const TG_WS_PROXY_PROCESS_NAME: &str = "tg-ws-proxy.exe";

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

fn ensure_tg_ws_proxy_runtime_dir() -> Result<std::path::PathBuf, String> {
    let dir = tg_ws_proxy_runtime_dir();
    std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    Ok(dir)
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
    unsafe {
        let handle = OpenProcess(
            PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION,
            false,
            pid,
        )
        .map_err(|error| format!("Failed to open process {pid}: {error}"))?;

        let result = TerminateProcess(handle, 1);
        let _ = CloseHandle(handle);
        result.map_err(|error| format!("Failed to terminate process {pid}: {error}"))
    }
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

fn get_tg_ws_proxy_status_inner() -> Result<TgWsProxyStatus, String> {
    ensure_tg_ws_proxy_runtime_dir()?;

    let stored_pid = TG_WS_PROXY_PID.load(Ordering::SeqCst);
    #[cfg(windows)]
    let running_pid = if stored_pid != 0 && is_process_running_by_pid(stored_pid) {
        Some(stored_pid)
    } else {
        let recovered = recover_running_pids().into_iter().next();
        if let Some(pid) = recovered {
            TG_WS_PROXY_PID.store(pid, Ordering::SeqCst);
        } else {
            TG_WS_PROXY_PID.store(0, Ordering::SeqCst);
            clear_stored_handle();
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

    if let Some(handle) = handle
        && handle
            .try_wait()
            .map_err(|error| format!("Failed to inspect tg-ws-proxy state: {error}"))?
            .is_none()
    {
        if let Err(error) = handle.kill() {
            stop_errors.push(format!("Failed to kill tg-ws-proxy: {error}"));
        }
        let _ = handle.wait_timeout(Duration::from_secs(2));
    }

    #[cfg(windows)]
    {
        for pid in recover_running_pids() {
            if let Err(error) = terminate_process_by_pid(pid) {
                stop_errors.push(format!(
                    "Failed to terminate tg-ws-proxy process {pid}: {error}"
                ));
            }
        }
    }

    TG_WS_PROXY_PID.store(0, Ordering::SeqCst);
    let status = get_tg_ws_proxy_status_inner()?;
    if status.running {
        stop_errors.push("tg-ws-proxy.exe всё ещё запущен после остановки".to_string());
    }

    if stop_errors.is_empty() {
        Ok(status)
    } else {
        Err(stop_errors.join(" | "))
    }
}

fn start_tg_ws_proxy_inner(port: u16, secret: String) -> Result<TgWsProxyStatus, String> {
    if port == 0 {
        return Err("Порт должен быть больше 0".to_string());
    }

    let secret = validate_tg_ws_proxy_secret(&secret)?;
    let binary_path = tg_ws_proxy_binary_path();
    if !binary_path.is_file() {
        return Err("tg-ws-proxy.exe не найден в resources/modules/tg-ws-proxy-rs".to_string());
    }

    ensure_tg_ws_proxy_runtime_dir()?;
    if let Err(error) = stop_tg_ws_proxy_inner() {
        return Err(format!(
            "Failed to stop existing tg-ws-proxy before restart: {error}"
        ));
    }

    let args = vec![
        "--host".to_string(),
        "127.0.0.1".to_string(),
        "--link-ip".to_string(),
        "127.0.0.1".to_string(),
        "--port".to_string(),
        port.to_string(),
        "--secret".to_string(),
        secret,
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
            return Err(message);
        }

        if std::time::Instant::now() >= startup_deadline {
            break;
        }

        std::thread::sleep(Duration::from_millis(100));
    }

    get_tg_ws_proxy_status_inner()
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
