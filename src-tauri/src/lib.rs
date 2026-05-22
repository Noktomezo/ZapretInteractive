mod commands;

use commands::{admin, binaries, config, discord_presence, dns, process, tg_proxy};
use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{
    Emitter, Manager,
    image::Image,
    menu::{CheckMenuItem, Menu, MenuItem, Submenu},
    tray::{MouseButton, TrayIconBuilder},
};
#[cfg(desktop)]
use tauri_plugin_autostart::ManagerExt;
#[cfg(target_os = "windows")]
use windows::{
    Win32::System::Registry::{HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RegGetValueW},
    core::w,
};

static CONNECTED: AtomicBool = AtomicBool::new(false);

pub(crate) fn sync_list_mode_ui(
    app: &tauri::AppHandle,
    mode: config::ListMode,
) -> Result<(), String> {
    let items = app.state::<ListModeItems>();
    items
        .ipset
        .set_checked(mode == config::ListMode::Ipset)
        .map_err(|e| e.to_string())?;
    items
        .exclude
        .set_checked(mode == config::ListMode::Exclude)
        .map_err(|e| e.to_string())?;
    app.emit("list-mode-changed", mode.to_string())
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn apply_list_mode(app: &tauri::AppHandle, mode: config::ListMode) {
    if CONNECTED.load(Ordering::SeqCst) {
        return;
    }

    let state = app.state::<config::AppState>();
    if let Err(error) = config::update_list_mode(app.clone(), mode, state) {
        eprintln!("Failed to apply list mode from tray: {error}");
        let _ = app.emit("list-mode-update-error", error);
    }
}

fn should_minimize_to_tray(state: &config::AppState) -> bool {
    state
        .config
        .lock()
        .map(|cfg| cfg.minimize_to_tray)
        .unwrap_or_else(|poisoned| poisoned.into_inner().minimize_to_tray)
}

#[cfg(target_os = "windows")]
pub(crate) fn get_windows_build_number() -> Option<u32> {
    let mut value = [0u16; 32];
    let mut value_size = (value.len() * std::mem::size_of::<u16>()) as u32;

    if unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            w!("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion"),
            w!("CurrentBuildNumber"),
            RRF_RT_REG_SZ,
            None,
            Some(value.as_mut_ptr().cast()),
            Some(&mut value_size),
        )
    }
    .is_err()
    {
        return None;
    }

    let value_len = (value_size as usize / std::mem::size_of::<u16>()).saturating_sub(1);
    let build_str = String::from_utf16_lossy(&value[..value_len]);
    build_str.trim().parse::<u32>().ok()
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn get_windows_build_number() -> Option<u32> {
    None
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(debug_assertions)]
    let devtools = tauri_plugin_devtools::init();

    #[cfg(debug_assertions)]
    let builder = tauri::Builder::default().plugin(devtools);

    #[cfg(not(debug_assertions))]
    let builder = tauri::Builder::default();

    builder
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            show_main_window(app);
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin({
            #[cfg(debug_assertions)]
            {
                tauri_plugin_prevent_default::debug()
            }
            #[cfg(not(debug_assertions))]
            {
                tauri_plugin_prevent_default::init()
            }
        })
        .setup(|app| {
            #[cfg(desktop)]
            app.handle().plugin(tauri_plugin_autostart::init(
                tauri_plugin_autostart::MacosLauncher::LaunchAgent,
                Some(vec!["--autostart"]),
            ))?;

            if let Err(error) = process::cleanup_orphaned_winws_on_startup() {
                eprintln!("Failed to clean up orphaned winws on startup: {error}");
            }
            if let Err(error) = tg_proxy::cleanup_orphaned_tg_ws_proxy_on_startup() {
                eprintln!("Failed to clean up orphaned tg-ws-proxy on startup: {error}");
            }
            if let Err(error) = dns::cleanup_orphaned_dns_proxy_on_startup() {
                eprintln!("Failed to clean up orphaned dnscrypt-proxy on startup: {error}");
            }

            let app_state = config::AppState::new()?;
            let list_mode = app_state
                .config
                .lock()
                .map(|cfg| cfg.list_mode)
                .unwrap_or_default();
            app.manage(app_state);

            let connect_item =
                MenuItem::with_id(app, "connect", "Подключиться", true, None::<&str>)?;
            let show_item = MenuItem::with_id(app, "show", "Показать", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Выход", true, None::<&str>)?;

            let ipset_item = CheckMenuItem::with_id(
                app,
                "listmode-ipset",
                "Только заблокированные",
                true,
                list_mode == config::ListMode::Ipset,
                None::<&str>,
            )?;
            let exclude_item = CheckMenuItem::with_id(
                app,
                "listmode-exclude",
                "Исключения",
                true,
                list_mode == config::ListMode::Exclude,
                None::<&str>,
            )?;

            let listmode_submenu =
                Submenu::with_items(app, "Режим списков", true, &[&ipset_item, &exclude_item])?;

            let menu = Menu::with_items(
                app,
                &[&connect_item, &listmode_submenu, &show_item, &quit_item],
            )?;

            let icon_bytes = include_bytes!("../icons/32x32.png") as &[u8];
            let icon = Image::from_bytes(icon_bytes).ok();

            let connect_item_clone = connect_item.clone();
            let ipset_item_clone = ipset_item.clone();
            let exclude_item_clone = exclude_item.clone();
            let listmode_submenu_clone = listmode_submenu.clone();

            let _tray = TrayIconBuilder::with_id("main")
                .tooltip("Zapret Interactive")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(move |app, event| match event.id.as_ref() {
                    "connect" => {
                        let _ = app.emit("tray-connect-toggle", ());
                    }
                    "listmode-ipset" => {
                        apply_list_mode(app, config::ListMode::Ipset);
                    }
                    "listmode-exclude" => {
                        apply_list_mode(app, config::ListMode::Exclude);
                    }
                    "show" => {
                        show_main_window(app);
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click { button, .. } = event
                        && button == MouseButton::Left
                    {
                        let app = tray.app_handle();
                        show_main_window(app);
                    }
                });

            if let Some(icon) = icon {
                _tray.icon(icon).build(app)?;
            } else {
                _tray.build(app)?;
            }

            if let Some(window) = app.get_webview_window("main") {
                let state = app.state::<config::AppState>();

                let window_clone = window.clone();
                let app_handle = app.handle().clone();
                let launch_to_tray = state
                    .config
                    .lock()
                    .map(|cfg| cfg.launch_to_tray)
                    .unwrap_or_else(|poisoned| poisoned.into_inner().launch_to_tray);
                if launch_to_tray && was_launched_from_autostart() {
                    let _ = window.hide();
                }
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        let state = app_handle.state::<config::AppState>();
                        if should_minimize_to_tray(&state) {
                            api.prevent_close();
                            let _ = window_clone.hide();
                        }
                    }
                });
            }

            app.manage(ConnectMenuItem(connect_item_clone));
            app.manage(ListModeItems {
                ipset: ipset_item_clone,
                exclude: exclude_item_clone,
                submenu: listmode_submenu_clone,
            });

            binaries::start_files_watcher(app.handle().clone())?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            admin::is_elevated,
            config::ensure_config_dir,
            config::load_config,
            config::get_builtin_config,
            config::save_config,
            config::reset_config,
            config::get_resources_directory,
            config::config_exists,
            config::resolve_placeholders,
            config::update_list_mode,
            binaries::verify_binaries,
            binaries::get_missing_critical_files,
            binaries::get_app_health_snapshot,
            binaries::ensure_managed_files,
            binaries::restore_hashes_from_disk,
            binaries::download_binaries,
            binaries::apply_core_file_updates,
            binaries::refresh_lists_if_stale,
            binaries::restore_default_filters,
            binaries::get_binary_path,
            binaries::get_winws_path,
            binaries::get_filters_path,
            binaries::get_reserved_filter_filenames,
            binaries::save_filter_file,
            binaries::load_filter_file,
            binaries::delete_filter_file,
            binaries::open_app_directory,
            binaries::open_filters_directory,
            discord_presence::sync_discord_presence,
            dns::get_dns_proxy_status,
            dns::start_dns_proxy,
            dns::stop_dns_proxy,
            dns::check_dns_provider_latency,
            tg_proxy::get_tg_ws_proxy_status,
            tg_proxy::start_tg_ws_proxy,
            tg_proxy::stop_tg_ws_proxy,
            process::start_winws,
            process::stop_winws,
            process::is_winws_running,
            process::kill_windivert_service,
            process::get_running_pid,
            process::check_and_recover_orphan,
            process::check_tcp_timestamps,
            process::enable_tcp_timestamps,
            set_connected_state,
            is_autostart_enabled,
            set_autostart_enabled,
            was_launched_from_autostart,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

struct ConnectMenuItem(MenuItem<tauri::Wry>);

struct ListModeItems {
    ipset: CheckMenuItem<tauri::Wry>,
    exclude: CheckMenuItem<tauri::Wry>,
    submenu: Submenu<tauri::Wry>,
}

#[tauri::command]
fn set_connected_state(app: tauri::AppHandle, connected: bool) -> Result<(), String> {
    let text = if connected {
        "Отключиться"
    } else {
        "Подключиться"
    };

    let item = app.state::<ConnectMenuItem>();
    item.0.set_text(text).map_err(|e| e.to_string())?;

    let list_mode_items = app.state::<ListModeItems>();
    list_mode_items
        .submenu
        .set_enabled(!connected)
        .map_err(|e| e.to_string())?;

    CONNECTED.store(connected, Ordering::SeqCst);

    Ok(())
}

#[tauri::command]
fn is_autostart_enabled(app: tauri::AppHandle) -> Result<bool, String> {
    #[cfg(desktop)]
    {
        app.autolaunch().is_enabled().map_err(|e| e.to_string())
    }

    #[cfg(not(desktop))]
    {
        let _ = app;
        Ok(false)
    }
}

#[tauri::command]
fn set_autostart_enabled(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    #[cfg(desktop)]
    {
        let autolaunch = app.autolaunch();
        if enabled {
            autolaunch.enable().map_err(|e| e.to_string())
        } else {
            autolaunch.disable().map_err(|e| e.to_string())
        }
    }

    #[cfg(not(desktop))]
    {
        let _ = (app, enabled);
        Ok(())
    }
}

#[tauri::command]
fn was_launched_from_autostart() -> bool {
    std::env::args().any(|arg| arg == "--autostart")
}
