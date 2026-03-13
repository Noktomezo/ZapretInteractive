mod commands;

use commands::{admin, binaries, config, process};
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
use window_vibrancy::{apply_acrylic, clear_acrylic};

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
fn apply_window_transparency(window: &tauri::WebviewWindow, enabled: bool) -> Result<(), String> {
    if enabled {
        apply_acrylic(window, None).map_err(|e| e.to_string())
    } else {
        clear_acrylic(window).map_err(|e| e.to_string())
    }
}

#[cfg(not(target_os = "windows"))]
fn apply_window_transparency(_window: &tauri::WebviewWindow, _enabled: bool) -> Result<(), String> {
    Ok(())
}

fn report_window_transparency_error(app: &tauri::AppHandle, window_label: &str, error: &str) {
    eprintln!("Failed to apply window transparency for '{window_label}': {error}");
    let _ = app.emit(
        "window-transparency-error",
        format!("{window_label}: {error}"),
    );
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
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
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
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                });

            if let Some(icon) = icon {
                _tray.icon(icon).build(app)?;
            } else {
                _tray.build(app)?;
            }

            if let Some(window) = app.get_webview_window("main") {
                let state = app.state::<config::AppState>();
                let window_acrylic_enabled = state
                    .config
                    .lock()
                    .map(|cfg| cfg.window_acrylic_enabled)
                    .unwrap_or_else(|poisoned| poisoned.into_inner().window_acrylic_enabled);
                if let Err(error) = apply_window_transparency(&window, window_acrylic_enabled) {
                    report_window_transparency_error(app.handle(), "main", &error);

                    if window_acrylic_enabled {
                        let mut fallback_config = match state.config.lock() {
                            Ok(cfg) => cfg.clone(),
                            Err(poisoned) => poisoned.into_inner().clone(),
                        };
                        fallback_config.window_acrylic_enabled = false;

                        if let Err(save_error) = config::save_config_to_disk(&fallback_config) {
                            eprintln!(
                                "Failed to persist disabled window transparency after startup error: {save_error}"
                            );
                        }

                        match state.config.lock() {
                            Ok(mut cfg) => *cfg = fallback_config,
                            Err(poisoned) => {
                                let mut cfg = poisoned.into_inner();
                                *cfg = fallback_config;
                            }
                        }
                    }
                }

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
            config::save_config,
            config::reset_config,
            config::get_zapret_directory,
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
            binaries::open_zapret_directory,
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
            set_window_transparency_enabled,
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

#[tauri::command]
fn set_window_transparency_enabled(
    app: tauri::AppHandle,
    state: tauri::State<'_, config::AppState>,
    enabled: bool,
) -> Result<(), String> {
    let current_config = state
        .config
        .lock()
        .map(|cfg| cfg.clone())
        .unwrap_or_else(|poisoned| poisoned.into_inner().clone());

    let mut next_config = current_config.clone();
    next_config.window_acrylic_enabled = enabled;

    config::save_config_to_disk(&next_config)?;

    {
        let mut cfg = state.config.lock().map_err(|e| e.to_string())?;
        *cfg = next_config.clone();
    }

    if let Some(window) = app.get_webview_window("main")
        && let Err(error) = apply_window_transparency(&window, enabled)
    {
        let rollback_result = (|| -> Result<(), String> {
            config::save_config_to_disk(&current_config)?;
            let mut cfg = state.config.lock().map_err(|e| e.to_string())?;
            *cfg = current_config.clone();
            Ok(())
        })();

        if let Err(rollback_error) = rollback_result {
            report_window_transparency_error(
                &app,
                "main",
                &format!(
                    "{error}; also failed to rollback window transparency config: {rollback_error}"
                ),
            );
            return Err(format!(
                "{error}; также не удалось откатить настройку: {rollback_error}"
            ));
        }

        report_window_transparency_error(&app, "main", &error);
        return Err(error);
    }

    Ok(())
}
