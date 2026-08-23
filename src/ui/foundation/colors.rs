//! Flexoki Color Scheme (Dynamic Theme Engine for Flexoki Dark & Flexoki Light)
//! Design reference: https://stephango.com/flexoki

#![allow(dead_code)]

use gpui::{Rgba, rgba};
use std::sync::atomic::{AtomicBool, Ordering};

/// Theme selection for the shared Flexoki palette.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ThemeMode {
    #[default]
    System,
    Dark,
    Light,
}

static IS_DARK_THEME: AtomicBool = AtomicBool::new(true);

pub fn detect_system_dark_mode() -> bool {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt as _;
        let output = duct::cmd(
            "reg.exe",
            [
                "query",
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize",
                "/v",
                "AppsUseLightTheme",
            ],
        )
        .before_spawn(|cmd| {
            cmd.creation_flags(0x0800_0000);
            Ok(())
        })
        .unchecked()
        .read();
        if let Ok(output) = output {
            return !output.contains("0x1");
        }
    }
    true
}

pub fn set_active_theme(mode: ThemeMode) {
    let is_dark = match mode {
        ThemeMode::System => detect_system_dark_mode(),
        ThemeMode::Dark => true,
        ThemeMode::Light => false,
    };
    IS_DARK_THEME.store(is_dark, Ordering::Relaxed);
    update_tray_menu_theme(is_dark);
}

#[cfg(windows)]
pub fn update_tray_menu_theme(is_dark: bool) {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn LoadLibraryA(lpLibFileName: *const u8) -> *mut std::ffi::c_void;
        fn GetProcAddress(
            hModule: *mut std::ffi::c_void,
            lpProcName: *const u8,
        ) -> *mut std::ffi::c_void;
    }

    // SAFETY: Loading system uxtheme.dll and invoking documented ordinals:
    // 135: SetPreferredAppMode (Windows 10 1809+)
    // 136: FlushMenuThemes (forces Win32 popup context menus to refresh their theme)
    unsafe {
        let uxtheme = LoadLibraryA(c"uxtheme.dll".as_ptr().cast::<u8>());
        if !uxtheme.is_null() {
            type SetPreferredAppModeFn = unsafe extern "system" fn(i32) -> i32;
            type FlushMenuThemesFn = unsafe extern "system" fn();

            let set_mode_ptr = GetProcAddress(uxtheme, 135 as *const u8);
            if !set_mode_ptr.is_null() {
                let set_mode: SetPreferredAppModeFn = std::mem::transmute(set_mode_ptr);
                let mode = if is_dark { 2 } else { 3 }; // 2 = ForceDark, 3 = ForceLight
                set_mode(mode);
            }

            let flush_ptr = GetProcAddress(uxtheme, 136 as *const u8);
            if !flush_ptr.is_null() {
                let flush: FlushMenuThemesFn = std::mem::transmute(flush_ptr);
                flush();
            }
        }
    }
}

#[cfg(not(windows))]
pub fn update_tray_menu_theme(_is_dark: bool) {}

pub fn is_dark() -> bool {
    IS_DARK_THEME.load(Ordering::Relaxed)
}

// Semantic tokens use the canonical Flexoki palette from assets/THEME.md.
#[inline(always)]
pub fn background() -> Rgba {
    black()
}

#[inline(always)]
pub fn foreground() -> Rgba {
    base_200()
}

#[inline(always)]
pub fn card() -> Rgba {
    base_950().opacity(if is_dark() { 0.94 } else { 0.92 })
}

#[inline(always)]
pub fn popover() -> Rgba {
    base_900().opacity(0.97)
}

#[inline(always)]
pub fn secondary() -> Rgba {
    base_900()
}

#[inline(always)]
pub fn muted() -> Rgba {
    base_850()
}

#[inline(always)]
pub fn muted_foreground() -> Rgba {
    base_500()
}

#[inline(always)]
pub fn border() -> Rgba {
    base_800()
}

#[inline(always)]
pub fn input() -> Rgba {
    base_850()
}

#[inline(always)]
pub fn success() -> Rgba {
    green()
}

#[inline(always)]
pub fn warning() -> Rgba {
    yellow()
}

#[inline(always)]
pub fn destructive() -> Rgba {
    red()
}

#[inline(always)]
pub fn accent() -> Rgba {
    yellow()
}

// Dynamic Theme Colors (Seamless Flexoki Dark / Light Mode)
#[inline(always)]
pub fn black() -> Rgba {
    if is_dark() {
        rgba(0x100f0fff)
    } else {
        rgba(0xfffcf0ff)
    }
}

#[inline(always)]
pub fn base_950() -> Rgba {
    if is_dark() {
        rgba(0x1c1b1aff)
    } else {
        rgba(0xf2f0e5ff)
    }
}

#[inline(always)]
pub fn base_900() -> Rgba {
    if is_dark() {
        rgba(0x282726ff)
    } else {
        rgba(0xe6e4d9ff)
    }
}

#[inline(always)]
pub fn base_850() -> Rgba {
    if is_dark() {
        rgba(0x343331ff)
    } else {
        rgba(0xdad8ceff)
    }
}

#[inline(always)]
pub fn base_800() -> Rgba {
    if is_dark() {
        rgba(0x403e3cff)
    } else {
        rgba(0xcecdc3ff)
    }
}

#[inline(always)]
pub fn base_700() -> Rgba {
    if is_dark() {
        rgba(0x575653ff)
    } else {
        rgba(0xb7b5acff)
    }
}

#[inline(always)]
pub fn base_600() -> Rgba {
    if is_dark() {
        rgba(0x6f6e69ff)
    } else {
        rgba(0x878580ff)
    }
}

#[inline(always)]
pub fn base_500() -> Rgba {
    if is_dark() {
        rgba(0x878580ff)
    } else {
        rgba(0x6f6e69ff)
    }
}

#[inline(always)]
pub fn base_300() -> Rgba {
    if is_dark() {
        rgba(0xb7b5acff)
    } else {
        rgba(0x575653ff)
    }
}

#[inline(always)]
pub fn base_200() -> Rgba {
    if is_dark() {
        rgba(0xcecdc3ff)
    } else {
        rgba(0x282726ff)
    }
}

#[inline(always)]
pub fn base_150() -> Rgba {
    if is_dark() {
        rgba(0xdad8ceff)
    } else {
        rgba(0x1c1b1aff)
    }
}

#[inline(always)]
pub fn base_100() -> Rgba {
    if is_dark() {
        rgba(0xe6e4d9ff)
    } else {
        rgba(0x100f0fff)
    }
}

#[inline(always)]
pub fn paper() -> Rgba {
    rgba(0xfffcf0ff)
}

/// Foreground for controls with a solid semantic accent background.
#[inline(always)]
pub fn accent_foreground() -> Rgba {
    if is_dark() { rgba(0x100f0fff) } else { paper() }
}

// Flexoki Accent Colors
#[inline(always)]
pub fn red_600() -> Rgba {
    rgba(0xaf3029ff)
}
#[inline(always)]
pub fn red_400() -> Rgba {
    rgba(0xd14d41ff)
}
#[inline(always)]
pub fn red() -> Rgba {
    if is_dark() { red_400() } else { red_600() }
}

#[inline(always)]
pub fn orange_600() -> Rgba {
    rgba(0xbc5215ff)
}
#[inline(always)]
pub fn orange_400() -> Rgba {
    rgba(0xda702cff)
}
#[inline(always)]
pub fn orange() -> Rgba {
    if is_dark() {
        orange_400()
    } else {
        orange_600()
    }
}

#[inline(always)]
pub fn yellow_600() -> Rgba {
    rgba(0xad8301ff)
}
#[inline(always)]
pub fn yellow_400() -> Rgba {
    rgba(0xd0a215ff)
}
#[inline(always)]
pub fn yellow() -> Rgba {
    if is_dark() {
        yellow_400()
    } else {
        yellow_600()
    }
}

#[inline(always)]
pub fn green_600() -> Rgba {
    rgba(0x66800bff)
}
#[inline(always)]
pub fn green_400() -> Rgba {
    rgba(0x879a39ff)
}
#[inline(always)]
pub fn green() -> Rgba {
    if is_dark() { green_400() } else { green_600() }
}

#[inline(always)]
pub fn cyan_600() -> Rgba {
    rgba(0x24837bff)
}
#[inline(always)]
pub fn cyan_400() -> Rgba {
    rgba(0x3aa99fff)
}
#[inline(always)]
pub fn cyan() -> Rgba {
    if is_dark() { cyan_400() } else { cyan_600() }
}

#[inline(always)]
pub fn blue_600() -> Rgba {
    rgba(0x205ea6ff)
}
#[inline(always)]
pub fn blue_400() -> Rgba {
    rgba(0x4385beff)
}
#[inline(always)]
pub fn blue() -> Rgba {
    if is_dark() { blue_400() } else { blue_600() }
}

#[inline(always)]
pub fn purple_600() -> Rgba {
    rgba(0x5e409dff)
}
#[inline(always)]
pub fn purple_400() -> Rgba {
    rgba(0x8b7ec8ff)
}
#[inline(always)]
pub fn purple() -> Rgba {
    if is_dark() {
        purple_400()
    } else {
        purple_600()
    }
}

#[inline(always)]
pub fn magenta_600() -> Rgba {
    rgba(0xa02f6fff)
}
#[inline(always)]
pub fn magenta_400() -> Rgba {
    rgba(0xce5d97ff)
}
#[inline(always)]
pub fn magenta() -> Rgba {
    if is_dark() {
        magenta_400()
    } else {
        magenta_600()
    }
}
