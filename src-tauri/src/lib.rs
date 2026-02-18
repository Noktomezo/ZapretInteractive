mod commands;

use commands::{admin, binaries, config, process};
use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{
    image::Image,
    menu::{CheckMenuItem, Menu, MenuItem, Submenu},
    tray::{TrayIconBuilder, MouseButton},
    Emitter, Manager,
};

static CONNECTED: AtomicBool = AtomicBool::new(false);

fn apply_list_mode(
    app: &tauri::AppHandle,
    mode: config::ListMode,
    ipset_checked: bool,
    exclude_checked: bool,
    event_payload: &str,
) {
    if !CONNECTED.load(Ordering::SeqCst) {
        if let Ok(mut cfg) = config::load_config() {
            cfg.list_mode = mode;
            if config::save_config(cfg).is_ok() {
                let items = app.state::<ListModeItems>();
                let _ = items.ipset.set_checked(ipset_checked);
                let _ = items.exclude.set_checked(exclude_checked);
                let _ = app.emit("list-mode-changed", event_payload);
            }
        }
    }
}

fn should_minimize_to_tray() -> bool {
    let config_path = config::get_config_path();
    if !config_path.exists() {
        return true;
    }
    fs::read_to_string(&config_path)
        .ok()
        .and_then(|content| serde_json::from_str::<serde_json::Value>(&content).ok())
        .and_then(|json| json.get("minimizeToTray").and_then(|v| v.as_bool()))
        .unwrap_or(true)
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
        .invoke_handler(tauri::generate_handler![
            admin::is_elevated,
            config::ensure_config_dir,
            config::load_config,
            config::save_config,
            config::reset_config,
            config::get_zapret_directory,
            config::resolve_placeholders,
            binaries::verify_binaries,
            binaries::download_binaries,
            binaries::get_binary_path,
            binaries::get_winws_path,
            binaries::get_filters_path,
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
        ])
        .setup(|app| {
            let connect_item = MenuItem::with_id(app, "connect", "Подключиться", true, None::<&str>)?;
            let show_item = MenuItem::with_id(app, "show", "Показать", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Выход", true, None::<&str>)?;

            let list_mode = config::load_config()
                .map(|cfg| cfg.list_mode)
                .unwrap_or_default();

            let ipset_item = CheckMenuItem::with_id(app, "listmode-ipset", "Только заблокированные", true, list_mode == config::ListMode::Ipset, None::<&str>)?;
            let exclude_item = CheckMenuItem::with_id(app, "listmode-exclude", "Исключения", true, list_mode == config::ListMode::Exclude, None::<&str>)?;

            let listmode_submenu = Submenu::with_items(app, "Режим списков", true, &[&ipset_item, &exclude_item])?;

            let menu = Menu::with_items(app, &[&connect_item, &listmode_submenu, &show_item, &quit_item])?;

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
                        apply_list_mode(app, config::ListMode::Ipset, true, false, "ipset");
                    }
                    "listmode-exclude" => {
                        apply_list_mode(app, config::ListMode::Exclude, false, true, "exclude");
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
                    if let tauri::tray::TrayIconEvent::Click { button, .. } = event {
                        if button == MouseButton::Left {
                            let app = tray.app_handle();
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    }
                });

            if let Some(icon) = icon {
                _tray.icon(icon).build(app)?;
            } else {
                _tray.build(app)?;
            }

            if let Some(window) = app.get_webview_window("main") {
                let window_clone = window.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        if should_minimize_to_tray() {
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

            Ok(())
        })
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
    CONNECTED.store(connected, Ordering::SeqCst);

    let text = if connected { "Отключиться" } else { "Подключиться" };

    let item = app.state::<ConnectMenuItem>();
    item.0.set_text(text).map_err(|e| e.to_string())?;

    let list_mode_items = app.state::<ListModeItems>();
    list_mode_items.submenu.set_enabled(!connected).map_err(|e| e.to_string())?;

    Ok(())
}