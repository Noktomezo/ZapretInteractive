use crate::config::get_zapret_dir;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

static RUNNING_PID: AtomicU32 = AtomicU32::new(0);

pub fn set_running_pid(pid: u32) {
    RUNNING_PID.store(pid, Ordering::SeqCst);
}

#[tauri::command]
pub fn start_winws(args: Vec<String>, tcp_ports: String, udp_ports: String) -> Result<u32, String> {
    let winws_path = get_zapret_dir().join("winws.exe");

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
    {
        Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| format!("Failed to kill winws.exe: {}", e))?;
    }

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
        let output = Command::new("tasklist")
            .args(["/FI", &format!("PID eq {}", pid)])
            .creation_flags(CREATE_NO_WINDOW)
            .output();

        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            return stdout.contains(&pid.to_string());
        }
        false
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
        Command::new("sc")
            .args(["stop", "WinDivert"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .ok();

        Command::new("sc")
            .args(["delete", "WinDivert"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| format!("Failed to remove WinDivert service: {}", e))?;
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
        let output = Command::new("netsh")
            .args(["interface", "tcp", "show", "global"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| format!("Failed to check TCP timestamps: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stdout_lower = stdout.to_lowercase();
        Ok(stdout_lower
            .lines()
            .any(|line| line.contains("rfc 1323") && line.contains("enabled")))
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
        Command::new("netsh")
            .args(["interface", "tcp", "set", "global", "timestamps=enabled"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| format!("Failed to enable TCP timestamps: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
pub fn check_and_recover_orphan() -> Option<u32> {
    #[cfg(windows)]
    {
        let output = Command::new("tasklist")
            .args(["/FI", "IMAGENAME eq winws.exe", "/FO", "CSV", "/NH"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .ok()?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        if stdout.contains("winws.exe") {
            for line in stdout.lines() {
                if line.contains("winws.exe") {
                    let parts: Vec<&str> = line.split(',').collect();
                    if parts.len() >= 2 {
                        let pid_str = parts[1].trim_matches('"');
                        if let Ok(pid) = pid_str.parse::<u32>() {
                            set_running_pid(pid);
                            return Some(pid);
                        }
                    }
                }
            }
        }
        None
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
