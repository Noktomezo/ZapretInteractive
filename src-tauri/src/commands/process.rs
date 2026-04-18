use super::config::{get_managed_resources_dir, get_runtime_data_dir};
use duct::{Expression, Handle, cmd};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use sysinfo::{Pid, ProcessesToUpdate, System};

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
use windows::Win32::Foundation::{
    CloseHandle, ERROR_FILE_NOT_FOUND, HANDLE, INVALID_HANDLE_VALUE, WAIT_FAILED, WAIT_OBJECT_0,
    WAIT_TIMEOUT, WIN32_ERROR,
};
#[cfg(windows)]
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
#[cfg(windows)]
use windows::Win32::System::Registry::{
    HKEY, HKEY_LOCAL_MACHINE, KEY_QUERY_VALUE, KEY_SET_VALUE, KEY_WRITE, REG_DWORD,
    REG_OPTION_NON_VOLATILE, RegCloseKey, RegCreateKeyExW, RegOpenKeyExW, RegQueryValueExW,
    RegSetValueExW,
};
#[cfg(windows)]
use windows::Win32::System::Services::{
    CloseServiceHandle, ControlService, DeleteService, OpenSCManagerW, OpenServiceW,
    QueryServiceStatus, SC_MANAGER_CONNECT, SERVICE_CONTROL_STOP, SERVICE_QUERY_STATUS,
    SERVICE_STATUS, SERVICE_STOP, SERVICE_STOPPED,
};
#[cfg(windows)]
use windows::Win32::System::Threading::{
    GetExitCodeProcess, OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
    PROCESS_SYNCHRONIZE, PROCESS_TERMINATE, QueryFullProcessImageNameW, TerminateProcess,
    WaitForSingleObject,
};
#[cfg(windows)]
use windows::core::{PCWSTR, PWSTR};

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[cfg(windows)]
const TCPIP_PARAMETERS_PATH: &str = "SYSTEM\\CurrentControlSet\\Services\\Tcpip\\Parameters";
#[cfg(windows)]
const TCP1323_OPTS_NAME: &str = "Tcp1323Opts";
#[cfg(windows)]
const TCP_TIMESTAMP_BIT: u32 = 0b10;
#[cfg(windows)]
const DRIVER_SERVICE_NAMES: &[&str] = &["WinDivert", "Monkey64", "Monkey"];
#[cfg(windows)]
const STILL_ACTIVE_EXIT_CODE: u32 = 259;
#[cfg(windows)]
const DELETE_ACCESS_MASK: u32 = 0x0001_0000;
const WINWS_PROCESS_NAME: &str = "winws.exe";

static RUNNING_PID: AtomicU32 = AtomicU32::new(0);
static RUNNING_HANDLE: Mutex<Option<Handle>> = Mutex::new(None);

fn winws_binary_path() -> PathBuf {
    get_managed_resources_dir().join(WINWS_PROCESS_NAME)
}

fn winws_pid_path() -> PathBuf {
    get_runtime_data_dir().join("winws.pid")
}

fn write_pid_file(path: &Path, pid: u32) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    std::fs::write(path, pid.to_string()).map_err(|error| error.to_string())
}

fn read_pid_file(path: &Path) -> Option<u32> {
    let content = std::fs::read_to_string(path).ok()?;
    content.trim().parse::<u32>().ok().filter(|pid| *pid != 0)
}

fn clear_pid_file(path: &Path) -> Result<(), String> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.to_string()),
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
fn is_benign_service_delete_error(code: i32) -> bool {
    matches!(code as u32, 0x80070430 | 0x80070424)
}

#[cfg(windows)]
fn is_benign_service_state_error(error: &str) -> bool {
    fn is_benign_error_code(code: u32) -> bool {
        matches!(code, 1060 | 1072 | 0x80070424 | 0x80070430)
    }

    let upper = error.to_ascii_uppercase();
    for token in upper.split(|c: char| !(c.is_ascii_hexdigit() || c == 'X')) {
        if token.is_empty() {
            continue;
        }

        if let Some(hex) = token.strip_prefix("0X") {
            if let Ok(code) = u32::from_str_radix(hex, 16)
                && is_benign_error_code(code)
            {
                return true;
            }
            continue;
        }

        if let Ok(code) = token.parse::<u32>()
            && is_benign_error_code(code)
        {
            return true;
        }
    }

    error.contains("does not exist") || error.contains("marked for deletion")
}

#[cfg(windows)]
fn to_wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
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
                    let wait_result = WaitForSingleObject(handle, 2_000);
                    let _ = CloseHandle(handle);

                    if wait_result == WAIT_OBJECT_0 {
                        return Ok(());
                    } else if wait_result == WAIT_TIMEOUT {
                        errors.push(format!(
                            "Process {pid} did not exit within 2000 ms after WinAPI termination"
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
fn is_process_running_by_pid(pid: u32) -> bool {
    unsafe {
        let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
            return false;
        };

        let expected_path = normalize_path_for_compare(&winws_binary_path());
        let actual_path = query_process_image_path(handle)
            .map(|path| normalize_path_for_compare(&path))
            .is_some_and(|path| path == expected_path);
        let mut exit_code = 0u32;
        let result = GetExitCodeProcess(handle, &mut exit_code).is_ok()
            && exit_code == STILL_ACTIVE_EXIT_CODE
            && actual_path;
        let _ = CloseHandle(handle);
        result
    }
}

#[cfg(windows)]
fn find_process_pid_by_name(process_name: &str) -> Option<u32> {
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).ok()?;
        if snapshot == INVALID_HANDLE_VALUE {
            return None;
        }

        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };

        let mut found = None;
        if Process32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                let len = entry
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(entry.szExeFile.len());
                let exe = String::from_utf16_lossy(&entry.szExeFile[..len]);
                if exe.eq_ignore_ascii_case(process_name) {
                    found = Some(entry.th32ProcessID);
                    break;
                }

                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }

        let _ = CloseHandle(snapshot);
        found
    }
}

#[cfg(windows)]
fn find_expected_winws_pid() -> Option<u32> {
    find_process_pid_by_name(WINWS_PROCESS_NAME).filter(|pid| is_process_running_by_pid(*pid))
}

#[cfg(windows)]
fn stop_and_delete_service(service_name: &str) -> Result<(), String> {
    unsafe {
        let manager = OpenSCManagerW(None, None, SC_MANAGER_CONNECT)
            .map_err(|e| format!("Failed to open Service Control Manager: {e}"))?;

        let service_name_w = to_wide(service_name);
        let service = OpenServiceW(
            manager,
            PCWSTR(service_name_w.as_ptr()),
            SERVICE_STOP | SERVICE_QUERY_STATUS | DELETE_ACCESS_MASK,
        );

        let service = match service {
            Ok(service) => service,
            Err(_) => {
                let _ = CloseServiceHandle(manager);
                return Ok(());
            }
        };

        let mut status = SERVICE_STATUS::default();
        let _ = ControlService(service, SERVICE_CONTROL_STOP, &mut status);
        let _ = QueryServiceStatus(service, &mut status);
        if status.dwCurrentState != SERVICE_STOPPED {
            std::thread::sleep(std::time::Duration::from_millis(300));
            let _ = QueryServiceStatus(service, &mut status);
        }

        let delete_result = DeleteService(service);
        let _ = CloseServiceHandle(service);
        let _ = CloseServiceHandle(manager);

        match delete_result {
            Ok(()) => Ok(()),
            Err(e) if is_benign_service_delete_error(e.code().0) => Ok(()),
            Err(e) => Err(format!("Failed to delete service {service_name}: {e}")),
        }
    }
}

#[cfg(windows)]
fn read_tcp1323_opts() -> Result<u32, String> {
    unsafe {
        let path = to_wide(TCPIP_PARAMETERS_PATH);
        let value_name = to_wide(TCP1323_OPTS_NAME);
        let mut key = HKEY::default();

        let open_result = RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            PCWSTR(path.as_ptr()),
            Some(0),
            KEY_QUERY_VALUE,
            &mut key,
        );

        if open_result != WIN32_ERROR(0) {
            return Err(format!(
                "Failed to open TCP parameters registry key: {open_result:?}"
            ));
        }

        let mut data: u32 = 0;
        let mut data_len = std::mem::size_of::<u32>() as u32;
        let result = RegQueryValueExW(
            key,
            PCWSTR(value_name.as_ptr()),
            None,
            None,
            Some((&mut data as *mut u32).cast::<u8>()),
            Some(&mut data_len),
        );

        let _ = RegCloseKey(key);

        if result == WIN32_ERROR(0) {
            Ok(data)
        } else if result == ERROR_FILE_NOT_FOUND {
            Ok(0)
        } else {
            Err(format!("Failed to read Tcp1323Opts: {result:?}"))
        }
    }
}

#[cfg(windows)]
fn write_tcp1323_opts(value: u32) -> Result<(), String> {
    unsafe {
        let path = to_wide(TCPIP_PARAMETERS_PATH);
        let value_name = to_wide(TCP1323_OPTS_NAME);
        let mut key = HKEY::default();

        let create_result = RegCreateKeyExW(
            HKEY_LOCAL_MACHINE,
            PCWSTR(path.as_ptr()),
            Some(0),
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE | KEY_SET_VALUE,
            None,
            &mut key,
            None,
        );

        if create_result != WIN32_ERROR(0) {
            return Err(format!(
                "Failed to open/create TCP parameters registry key: {create_result:?}"
            ));
        }

        let bytes = value.to_ne_bytes();
        let result = RegSetValueExW(
            key,
            PCWSTR(value_name.as_ptr()),
            Some(0),
            REG_DWORD,
            Some(&bytes),
        );

        let _ = RegCloseKey(key);

        if result == WIN32_ERROR(0) {
            Ok(())
        } else {
            Err(format!("Failed to write Tcp1323Opts: {result:?}"))
        }
    }
}

pub fn set_running_pid(pid: u32) {
    RUNNING_PID.store(pid, Ordering::SeqCst);
    if pid == 0 {
        let _ = clear_pid_file(&winws_pid_path());
    } else {
        let _ = write_pid_file(&winws_pid_path(), pid);
    }
}

pub(crate) fn cleanup_orphaned_winws_on_startup() -> Result<(), String> {
    let pid_path = winws_pid_path();
    let Some(pid) = read_pid_file(&pid_path) else {
        return Ok(());
    };

    if is_process_running_by_pid(pid) {
        terminate_process_by_pid(pid)?;
    }

    RUNNING_PID.store(0, Ordering::SeqCst);
    clear_pid_file(&pid_path)?;
    kill_windivert_service()?;
    Ok(())
}

#[tauri::command]
pub fn start_winws(args: Vec<String>, tcp_ports: String, udp_ports: String) -> Result<u32, String> {
    let winws_path = winws_binary_path();

    if !winws_path.exists() {
        return Err("winws.exe not found. Please download binaries first.".to_string());
    }

    let mut full_args: Vec<String> = vec![
        format!("--wf-tcp={}", tcp_ports),
        format!("--wf-udp={}", udp_ports),
    ];
    full_args.extend(args);

    let handle = configure_expression(cmd(winws_path.to_string_lossy().into_owned(), full_args))
        .start()
        .map_err(|e| format!("Failed to start winws.exe: {e}"))?;
    let pid = handle
        .pids()
        .into_iter()
        .next()
        .ok_or_else(|| "Failed to get winws.exe PID from duct handle".to_string())?;

    let mut running_handle = RUNNING_HANDLE.lock().map_err(|e| e.to_string())?;
    *running_handle = Some(handle);
    set_running_pid(pid);

    Ok(pid)
}

#[tauri::command]
pub fn stop_winws() -> Result<(), String> {
    let pid = RUNNING_PID.load(Ordering::SeqCst);
    let handle = RUNNING_HANDLE.lock().map_err(|e| e.to_string())?.take();

    if pid == 0 && handle.is_none() {
        return Ok(());
    }

    if let Some(handle) = handle {
        let wait_result = match handle.try_wait() {
            Ok(wait_result) => wait_result,
            Err(error) => {
                if let Ok(mut running_handle) = RUNNING_HANDLE.lock() {
                    *running_handle = Some(handle);
                }
                return Err(format!("Failed to inspect winws.exe state: {error}"));
            }
        };

        if wait_result.is_none() {
            if let Err(error) = handle.kill() {
                #[cfg(windows)]
                {
                    match terminate_process_by_pid(pid) {
                        Ok(()) => {}
                        Err(fallback_error) => {
                            if let Ok(mut running_handle) = RUNNING_HANDLE.lock() {
                                *running_handle = Some(handle);
                            }
                            return Err(format!(
                                "Failed to kill winws.exe: {error}; fallback by PID failed: {fallback_error}"
                            ));
                        }
                    }
                }

                #[cfg(not(windows))]
                {
                    if let Ok(mut running_handle) = RUNNING_HANDLE.lock() {
                        *running_handle = Some(handle);
                    }
                    return Err(format!("Failed to kill winws.exe: {error}"));
                }
            }
            let _ = handle.wait_timeout(Duration::from_secs(2));
        }
    } else {
        #[cfg(windows)]
        terminate_process_by_pid(pid)?;

        #[cfg(not(windows))]
        {
            Command::new("kill")
                .args(["-9", &pid.to_string()])
                .output()
                .map_err(|e| format!("Failed to kill winws.exe: {}", e))?;
        }
    }

    set_running_pid(0);
    kill_windivert_service()?;

    Ok(())
}

#[tauri::command]
pub fn is_winws_running() -> bool {
    let pid = RUNNING_PID.load(Ordering::SeqCst);

    if pid == 0 {
        return false;
    }

    #[cfg(windows)]
    {
        is_process_running_by_pid(pid)
    }

    #[cfg(not(windows))]
    {
        Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
}

#[tauri::command]
pub fn kill_windivert_service() -> Result<(), String> {
    #[cfg(windows)]
    {
        let mut errors = Vec::new();
        for service_name in DRIVER_SERVICE_NAMES {
            if let Err(error) = stop_and_delete_service(service_name) {
                if is_benign_service_state_error(&error) {
                    eprintln!("Non-fatal driver service state for {service_name}: {error}");
                    continue;
                }
                errors.push(format!("{service_name}: {error}"));
            }
        }

        if !errors.is_empty() {
            return Err(errors.join("; "));
        }
    }

    Ok(())
}

#[tauri::command]
pub fn get_running_pid() -> u32 {
    RUNNING_PID.load(Ordering::SeqCst)
}

#[tauri::command]
pub fn check_tcp_timestamps() -> Result<bool, String> {
    #[cfg(windows)]
    {
        let value = read_tcp1323_opts()?;
        Ok((value & TCP_TIMESTAMP_BIT) != 0)
    }

    #[cfg(not(windows))]
    {
        Ok(true)
    }
}

#[tauri::command]
pub fn enable_tcp_timestamps() -> Result<(), String> {
    #[cfg(windows)]
    {
        let current = read_tcp1323_opts()?;
        write_tcp1323_opts(current | TCP_TIMESTAMP_BIT)?;
    }

    Ok(())
}

#[tauri::command]
pub fn check_and_recover_orphan() -> Option<u32> {
    #[cfg(windows)]
    {
        let pid = find_expected_winws_pid()?;
        set_running_pid(pid);
        Some(pid)
    }

    #[cfg(not(windows))]
    {
        let output = Command::new("pgrep").arg("winws").output().ok()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        if let Ok(pid) = stdout.trim().parse::<u32>() {
            set_running_pid(pid);
            return Some(pid);
        }
        None
    }
}
