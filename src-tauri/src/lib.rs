mod commands;

use commands::{admin, binaries, config, process};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Emitter, Manager,
};

static CONNECTED: AtomicBool = AtomicBool::new(false);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
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
            let menu = Menu::with_items(app, &[&connect_item, &show_item, &quit_item])?;

            let icon = Image::from_path("icons/icon.ico")
                .or_else(|_| Image::from_path("icons/100x100.png"))
                .ok();

            let connect_item_clone = connect_item.clone();

            let _tray = TrayIconBuilder::with_id("main")
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(move |app, event| match event.id.as_ref() {
                    "connect" => {
                        let _ = app.emit("tray-connect-toggle", ());
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
                    if let tauri::tray::TrayIconEvent::Click { .. } = event {
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

            app.manage(ConnectMenuItem(connect_item_clone));

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

struct ConnectMenuItem(MenuItem<tauri::Wry>);

#[tauri::command]
fn set_connected_state(app: tauri::AppHandle, connected: bool) -> Result<(), String> {
    CONNECTED.store(connected, Ordering::SeqCst);
    
    let text = if connected { "Отключиться" } else { "Подключиться" };
    
    let item = app.state::<ConnectMenuItem>();
    item.0.set_text(text).map_err(|e| e.to_string())?;
    
    Ok(())
}