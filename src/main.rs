#![cfg_attr(windows, windows_subsystem = "windows")]

#[macro_use]
extern crate rust_i18n;

i18n!("src/locales", fallback = "ru");

use gpui::*;
use rust_embed::RustEmbed;

mod app;
mod app_state;
mod domain;
mod faulty_terminal;
mod pages;
mod services;
#[cfg(windows)]
mod tray;
pub mod ui;

use app::AppView;
use app_state::AppState;
use services::file_watcher::FileWatchEvent;
use services::single_instance::SingleInstanceGuard;

#[cfg(debug_assertions)]
const APP_TITLE: &str = "Zapret Interactive (Dev)";
#[cfg(not(debug_assertions))]
const APP_TITLE: &str = "Zapret Interactive";

#[cfg(debug_assertions)]
const APP_IDENTITY: &str = "com.noktomezo.zapret-interactive.dev";
#[cfg(not(debug_assertions))]
const APP_IDENTITY: &str = "com.noktomezo.zapret-interactive";

#[derive(RustEmbed)]
#[folder = "assets"]
#[include = "icons/**/*"]
#[include = "fonts/**/*"]
struct EmbeddedAssets;

impl AssetSource for EmbeddedAssets {
    fn load(&self, path: &str) -> Result<Option<std::borrow::Cow<'static, [u8]>>> {
        Ok(Self::get(path).map(|asset| asset.data))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(Self::iter()
            .filter(|asset| asset.starts_with(path))
            .map(SharedString::from)
            .collect())
    }
}

fn load_embedded_fonts(cx: &App) {
    let font_paths = [
        "fonts/IBM Plex Mono/IBMPlexMono-Regular.ttf",
        "fonts/IBM Plex Mono/IBMPlexMono-Medium.ttf",
        "fonts/IBM Plex Mono/IBMPlexMono-SemiBold.ttf",
        "fonts/IBM Plex Mono/IBMPlexMono-Bold.ttf",
        "fonts/IBM Plex Mono/IBMPlexMono-Italic.ttf",
        "fonts/IBM Plex Mono/IBMPlexMono-MediumItalic.ttf",
        "fonts/IBM Plex Mono/IBMPlexMono-SemiBoldItalic.ttf",
        "fonts/IBM Plex Mono/IBMPlexMono-BoldItalic.ttf",
        "fonts/IBM Plex Sans/IBMPlexSans-VariableFont_wdth,wght.ttf",
        "fonts/IBM Plex Sans/IBMPlexSans-Italic-VariableFont_wdth,wght.ttf",
    ];
    let mut fonts = Vec::new();
    for path in font_paths {
        if let Some(asset) = EmbeddedAssets::get(path) {
            fonts.push(asset.data);
        }
    }
    if !fonts.is_empty()
        && let Err(err) = cx.text_system().add_fonts(fonts)
    {
        eprintln!("Failed to register embedded fonts: {err:#}");
    }
}

fn main() {
    std::panic::set_hook(Box::new(|info| {
        let backtrace = std::backtrace::Backtrace::force_capture();
        let msg = format!("PANIC: {info}\nBacktrace:\n{backtrace}\n");
        eprintln!("{msg}");
        let _panic_log_result = std::fs::write("panic.log", msg);
    }));

    let _instance_guard = match SingleInstanceGuard::acquire() {
        Some(guard) => guard,
        None => {
            #[cfg(windows)]
            tray::restore_main_window();
            return;
        }
    };

    let launched_from_autostart = std::env::args().any(|argument| argument == "--autostart");
    Application::with_platform(gpui_platform::current_platform(false))
        .with_assets(EmbeddedAssets)
        .run(move |cx: &mut App| {
            load_embedded_fonts(cx);
            rust_i18n::set_locale(ui::foundation::i18n::detect_system_language());
            ui::components::text_input::init(cx);
            ui::components::text_area::init(cx);
            ui::components::cursor_tooltip::init(cx);
            ui::foundation::hover_motion::init(cx);
            pages::init(cx);
            cx.set_app_identity(APP_IDENTITY, APP_TITLE);
            let state = match AppState::load() {
                Ok(state) => cx.new(|_| state),
                Err(error) => {
                    let message = format!("не удалось запустить Zapret Interactive: {error:#}");
                    eprintln!("{message}");
                    if let Err(log_error) = std::fs::write("startup-error.log", &message) {
                        eprintln!("не удалось записать startup-error.log: {log_error}");
                    }
                    cx.quit();
                    return;
                }
            };

            match state.read(cx).start_files_watcher() {
                Ok(mut events) => {
                    let watched_state = state.downgrade();
                    cx.spawn(async move |cx| {
                        while let Some(event) = events.recv().await {
                            if watched_state
                                .update(cx, |state, cx| match event {
                                    FileWatchEvent::Changed => {
                                        state.maintain_managed_files(false, cx);
                                    }
                                    FileWatchEvent::Error(error) => {
                                        state.log(&error);
                                        cx.notify();
                                    }
                                })
                                .is_err()
                            {
                                break;
                            }
                        }
                    })
                    .detach();
                }
                Err(error) => state.update(cx, |state, cx| {
                    state.log(&format!("Не удалось запустить watcher файлов: {error:#}"));
                    cx.notify();
                }),
            }

            let state_for_bg = state.clone();
            cx.spawn(async move |cx| {
                state_for_bg.update(cx, |state, cx| {
                    state.maintain_managed_files(true, cx);
                    if state.config.app_auto_updates_enabled {
                        state.check_app_updates(cx);
                    }
                });
            })
            .detach();

            let periodic_state = state.downgrade();
            cx.spawn(async move |cx| {
                loop {
                    cx.background_executor()
                        .timer(std::time::Duration::from_secs(3 * 60 * 60))
                        .await;
                    if periodic_state
                        .update(cx, |state, cx| state.maintain_managed_files(true, cx))
                        .is_err()
                    {
                        break;
                    }
                }
            })
            .detach();

            let bounds = Bounds::centered(None, size(px(900.0), px(700.0)), cx);
            let state_for_window = state.clone();
            let window = cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    window_min_size: Some(size(px(900.0), px(700.0))),
                    titlebar: None,
                    window_background: WindowBackgroundAppearance::Blurred,
                    kind: WindowKind::Normal,
                    window_decorations: Some(WindowDecorations::Server),
                    ..Default::default()
                },
                move |window, cx| {
                    window.set_window_title(APP_TITLE);
                    let config = state_for_window.read(cx).config.clone();
                    if launched_from_autostart && config.connect_on_autostart {
                        state_for_window.update(cx, |state, cx| state.toggle_connection(cx));
                    }
                    AppView::new(state_for_window.clone(), window, cx)
                },
            );
            match window {
                Ok(window) => {
                    #[cfg(windows)]
                    {
                        if let Err(error) = tray::install(state.clone(), window, cx) {
                            eprintln!("не удалось создать трей: {error:#}");
                        }
                        if launched_from_autostart && state.read(cx).config.launch_to_tray {
                            tray::hide_main_window();
                        }
                    }
                }
                Err(error) => eprintln!("не удалось открыть окно: {error}"),
            }
        });
}
