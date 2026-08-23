use std::sync::mpsc;
use std::time::Duration;

use anyhow::{Context as _, Result};
use gpui::*;
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, Submenu};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, SW_HIDE, SW_RESTORE, SetForegroundWindow, ShowWindow,
};
use windows::core::w;

use crate::app::AppView;
use crate::app_state::AppState;
use crate::domain::{ConnectionStatus, ListMode};

#[cfg(debug_assertions)]
const WINDOW_TITLE: &str = "Zapret Interactive (Dev)";
#[cfg(not(debug_assertions))]
const WINDOW_TITLE: &str = "Zapret Interactive";

#[cfg(debug_assertions)]
const WINDOW_CLASS_OR_TITLE: windows::core::PCWSTR = w!("Zapret Interactive (Dev)");
#[cfg(not(debug_assertions))]
const WINDOW_CLASS_OR_TITLE: windows::core::PCWSTR = w!("Zapret Interactive");

#[derive(Clone, Copy)]
enum TrayAction {
    Connect,
    ListMode(ListMode),
    Show,
    Quit,
}

struct TrayController {
    _icon: TrayIcon,
    connect: MenuItem,
    ipset: CheckMenuItem,
    exclude: CheckMenuItem,
}

impl TrayController {
    fn sync(&self, status: ConnectionStatus, mode: ListMode) {
        self.connect
            .set_text(if status == ConnectionStatus::Connected {
                t!("tray.disconnect")
            } else {
                t!("tray.connect")
            });
        self.ipset.set_checked(mode == ListMode::Ipset);
        self.exclude.set_checked(mode == ListMode::Exclude);
    }
}

pub fn install(state: Entity<AppState>, window: WindowHandle<AppView>, cx: &mut App) -> Result<()> {
    crate::ui::foundation::colors::update_tray_menu_theme(crate::ui::foundation::colors::is_dark());
    let connect_text = t!("tray.connect");
    let show_text = t!("tray.show");
    let quit_text = t!("tray.quit");
    let ipset_text = t!("home.mode_ipset");
    let exclude_text = t!("home.mode_exclude");

    let connect = MenuItem::with_id("connect", &connect_text, true, None);
    let show = MenuItem::with_id("show", &show_text, true, None);
    let quit = MenuItem::with_id("quit", &quit_text, true, None);
    let mode = state.read(cx).config.list_mode;
    let ipset = CheckMenuItem::with_id(
        "listmode-ipset",
        &ipset_text,
        true,
        mode == ListMode::Ipset,
        None,
    );
    let exclude = CheckMenuItem::with_id(
        "listmode-exclude",
        &exclude_text,
        true,
        mode == ListMode::Exclude,
        None,
    );
    let list_mode = Submenu::with_items("Режим списков", true, &[&ipset, &exclude])
        .context("не удалось создать меню режима списков")?;
    let menu = Menu::with_items(&[&connect, &list_mode, &show, &quit])
        .context("не удалось создать меню трея")?;
    let (pixels, width, height) = tray_pixels()?;
    let icon = Icon::from_rgba(pixels, width, height).context("не удалось создать иконку трея")?;
    let tray = TrayIconBuilder::new()
        .with_id("main")
        .with_tooltip(WINDOW_TITLE)
        .with_menu(Box::new(menu))
        .with_menu_on_left_click(false)
        .with_icon(icon)
        .build()
        .context("не удалось создать системный трей")?;

    let (sender, receiver) = mpsc::channel();
    let menu_sender = sender.clone();
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        let action = match event.id().as_ref() {
            "connect" => Some(TrayAction::Connect),
            "listmode-ipset" => Some(TrayAction::ListMode(ListMode::Ipset)),
            "listmode-exclude" => Some(TrayAction::ListMode(ListMode::Exclude)),
            "show" => Some(TrayAction::Show),
            "quit" => Some(TrayAction::Quit),
            _ => None,
        };
        if let Some(action) = action {
            let _intentionally_ignored = menu_sender.send(action);
        }
    }));
    TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
        if matches!(
            event,
            TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            }
        ) {
            let _intentionally_ignored = sender.send(TrayAction::Show);
        }
    }));

    let controller = TrayController {
        _icon: tray,
        connect,
        ipset,
        exclude,
    };
    let timer = cx.background_executor().clone();
    cx.spawn(async move |cx| {
        loop {
            while let Ok(action) = receiver.try_recv() {
                match action {
                    TrayAction::Connect => {
                        state.update(cx, |state, cx| state.toggle_connection(cx));
                    }
                    TrayAction::ListMode(mode) => {
                        state.update(cx, |state, cx| {
                            if state.status == ConnectionStatus::Disconnected {
                                state.set_list_mode(mode, cx);
                            }
                        });
                    }
                    TrayAction::Show => show_main_window(&window, cx),
                    TrayAction::Quit => cx.update(|cx| cx.quit()),
                }
            }
            let (status, mode) =
                state.read_with(cx, |state, _| (state.status, state.config.list_mode));
            controller.sync(status, mode);
            timer.timer(Duration::from_millis(100)).await;
        }
    })
    .detach();
    Ok(())
}

pub fn hide_main_window() {
    // SAFETY: GPUI does not expose window hiding; FindWindowW only reads OS window state
    // and ShowWindow is called with the valid HWND returned by Windows.
    if let Ok(window) = unsafe { FindWindowW(None, WINDOW_CLASS_OR_TITLE) } {
        let _was_visible = unsafe { ShowWindow(window, SW_HIDE) };
    }
}

fn show_main_window(window: &WindowHandle<AppView>, cx: &mut AsyncApp) {
    restore_main_window();
    let _update = window.update(cx, |_, window, _| window.activate_window());
}

pub fn restore_main_window() {
    // SAFETY: GPUI does not expose restoring a hidden window; the HWND is validated by
    // FindWindowW before it is passed back to Windows.
    if let Ok(native) = unsafe { FindWindowW(None, WINDOW_CLASS_OR_TITLE) } {
        let _was_visible = unsafe { ShowWindow(native, SW_RESTORE) };
        let _foreground_result = unsafe { SetForegroundWindow(native) };
    }
}

#[cfg(debug_assertions)]
const TRAY_ICON_BYTES: &[u8] = include_bytes!("../assets/app-dev-tray.png");
#[cfg(not(debug_assertions))]
const TRAY_ICON_BYTES: &[u8] = include_bytes!("../assets/app-tray.png");

fn tray_pixels() -> Result<(Vec<u8>, u32, u32)> {
    let image = image::load_from_memory_with_format(TRAY_ICON_BYTES, image::ImageFormat::Png)
        .context("не удалось декодировать оригинальную иконку")?
        .to_rgba8();
    let (width, height) = image.dimensions();
    Ok((image.into_raw(), width, height))
}

#[cfg(test)]
mod tests {
    #[test]
    fn tray_icon_has_rgba_pixel_count() {
        let (pixels, width, height) = super::tray_pixels().expect("embedded icon is valid");
        let width = usize::try_from(width).expect("icon width fits usize");
        let height = usize::try_from(height).expect("icon height fits usize");
        assert_eq!(pixels.len(), width * height * 4);
    }
}
