use crate::config::get_managed_resources_dir;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
use windows::Win32::Foundation::{
    CloseHandle, ERROR_FILE_NOT_FOUND, INVALID_HANDLE_VALUE, WIN32_ERROR,
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
    GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE,
    TerminateProcess,
};
#[cfg(windows)]
use windows::core::PCWSTR;

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

static RUNNING_PID: AtomicU32 = AtomicU32::new(0);

#[cfg(windows)]
fn is_benign_service_delete_error(code: i32) -> bool {
    matches!(code as u32, 0x80070430 | 0x80070424)
}

#[cfg(windows)]
fn is_benign_service_state_error(error: &str) -> bool {
    error.contains("does not exist") || error.contains("marked for deletion")
}

#[cfg(windows)]
fn to_wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(windows)]
fn terminate_process_by_pid(pid: u32) -> Result<(), String> {
    unsafe {
        let handle = OpenProcess(
            PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION,
            false,
            pid,
        )
        .map_err(|e| format!("Failed to open process {pid}: {e}"))?;

        let result = TerminateProcess(handle, 1);
        let _ = CloseHandle(handle);

        result.map_err(|e| format!("Failed to terminate process {pid}: {e}"))
    }
}

#[cfg(windows)]
fn is_process_running_by_pid(pid: u32) -> bool {
    unsafe {
        let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
            return false;
        };

        let mut exit_code = 0u32;
        let result = GetExitCodeProcess(handle, &mut exit_code).is_ok()
            && exit_code == STILL_ACTIVE_EXIT_CODE;
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
}

#[tauri::command]
pub fn start_winws(args: Vec<String>, tcp_ports: String, udp_ports: String) -> Result<u32, String> {
    let winws_path = get_managed_resources_dir().join("winws.exe");

    if !winws_path.exists() {
        return Err("winws.exe not found. Please download binaries first.".to_string());
    }

    let mut full_args: Vec<String> = vec![
        format!("--wf-tcp={}", tcp_ports),
        format!("--wf-udp={}", udp_ports),
    ];
    full_args.extend(args);

    let child = Command::new(&winws_path)
        .args(&full_args)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| format!("Failed to start winws.exe: {}", e))?;

    let pid = child.id();
    RUNNING_PID.store(pid, Ordering::SeqCst);

    Ok(pid)
}

#[tauri::command]
pub fn stop_winws() -> Result<(), String> {
    let pid = RUNNING_PID.load(Ordering::SeqCst);

    if pid == 0 {
        return Ok(());
    }

    #[cfg(windows)]
    terminate_process_by_pid(pid)?;

    #[cfg(not(windows))]
    {
        Command::new("kill")
            .args(["-9", &pid.to_string()])
            .output()
            .map_err(|e| format!("Failed to kill winws.exe: {}", e))?;
    }

    RUNNING_PID.store(0, Ordering::SeqCst);
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
        let pid = find_process_pid_by_name("winws.exe")?;
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
