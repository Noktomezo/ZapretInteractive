mod commands;

use commands::{admin, binaries, config, process};
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager,
};

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
        ])
        .setup(|app| {
            let show_item = MenuItem::with_id(app, "show", "Показать", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Выход", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            let _tray = TrayIconBuilder::with_id("main")
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id.as_ref() {
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
                .build(app)?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}