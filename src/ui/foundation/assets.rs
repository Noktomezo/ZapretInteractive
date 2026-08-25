use gpui::{AssetSource, SharedString};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "assets"]
#[include = "icons/**/*"]
#[include = "fonts/**/*"]
pub struct EmbeddedAssets;

impl AssetSource for EmbeddedAssets {
    fn load(&self, path: &str) -> gpui::Result<Option<std::borrow::Cow<'static, [u8]>>> {
        Ok(Self::get(path).map(|asset| asset.data))
    }

    fn list(&self, path: &str) -> gpui::Result<Vec<SharedString>> {
        Ok(Self::iter()
            .filter(|asset| asset.starts_with(path))
            .map(SharedString::from)
            .collect())
    }
}

pub fn resolve_asset_path(rel_path: &str) -> String {
    rel_path
        .strip_prefix("assets/")
        .unwrap_or(rel_path)
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::{EmbeddedAssets, resolve_asset_path};
    use gpui::AssetSource as _;

    #[test]
    fn runtime_assets_are_embedded() {
        let path = resolve_asset_path("assets/icons/audio-waveform.svg");
        assert!(
            EmbeddedAssets
                .load(&path)
                .expect("asset lookup works")
                .is_some()
        );
    }

    #[test]
    fn all_settings_and_ui_icons_exist() {
        let icons = [
            "icons/house.svg",
            "icons/boxes.svg",
            "icons/layers.svg",
            "icons/funnel.svg",
            "icons/file-code.svg",
            "icons/logs.svg",
            "icons/settings.svg",
            "icons/info.svg",
            "icons/window-minimize.svg",
            "icons/window-restore.svg",
            "icons/window-maximize.svg",
            "icons/window-close.svg",
            "icons/refresh-cw.svg",
            "icons/cloud-download.svg",
            "icons/panel-left-close.svg",
            "icons/panel-left-open.svg",
            "icons/circle-off.svg",
            "icons/gamepad-2.svg",
            "icons/headphones.svg",
            "icons/clapperboard.svg",
            "icons/trophy.svg",
            "icons/chevron-right.svg",
            "icons/package.svg",
            "icons/user-round.svg",
            "icons/external-link.svg",
            "icons/download.svg",
            "icons/shield.svg",
            "icons/grip-vertical.svg",
            "icons/grip-horizontal.svg",
            "icons/user-round-plus.svg",
            "icons/pencil.svg",
            "icons/brush-cleaning.svg",
            "icons/trash-2.svg",
            "icons/plus.svg",
            "icons/arrow-left.svg",
            "icons/rotate-ccw.svg",
            "icons/shield-check.svg",
            "icons/network.svg",
            "icons/route.svg",
            "icons/send.svg",
            "icons/power.svg",
            "icons/copy.svg",
            "icons/palette.svg",
            "icons/bell-ring.svg",
            "icons/file-check.svg",
            "icons/hash.svg",
            "icons/app-window.svg",
            "icons/plug-zap.svg",
            "icons/minimize-2.svg",
            "icons/router.svg",
            "icons/arrow-left-right.svg",
            "icons/radar.svg",
            "icons/laptop.svg",
            "icons/sun.svg",
            "icons/moon-star.svg",
            "icons/check.svg",
            "icons/chevron-down.svg",
            "icons/chevron-up.svg",
            "icons/arrow-up.svg",
            "icons/circle-arrow-up.svg",
            "icons/triangle-alert.svg",
            "icons/circle-alert.svg",
            "icons/globe.svg",
            "icons/glass-shine.svg",
        ];
        let mut missing = Vec::new();
        for icon in icons {
            let path = resolve_asset_path(icon);
            if EmbeddedAssets.load(&path).expect("lookup works").is_none() {
                missing.push(icon);
            }
        }
        assert!(missing.is_empty(), "Missing icons: {missing:?}");
    }

    fn collect_source_files(dir: &std::path::Path, acc: &mut Vec<std::path::PathBuf>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.is_dir() {
                    collect_source_files(&path, acc);
                } else if path.extension().is_some_and(|ext| ext == "rs") {
                    acc.push(path);
                }
            }
        }
    }

    fn assert_no_duplicate_modifiers(method: &str) {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let src_dir = manifest_dir.join("src");
        let mut files = Vec::new();
        collect_source_files(&src_dir, &mut files);
        assert!(!files.is_empty(), "Must find Rust source files in src/");

        for path in files {
            let content = std::fs::read_to_string(&path).expect("read source file");
            let mut current_chain = String::new();
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with(".child(")
                    || trimmed.starts_with(".children(")
                    || trimmed.starts_with("div()")
                    || trimmed.starts_with("svg()")
                    || trimmed.ends_with(';')
                {
                    let count = current_chain.matches(method).count();
                    assert!(
                        count <= 1,
                        "Found {count} {method} calls on same element in {}: \n{}",
                        path.strip_prefix(manifest_dir).unwrap_or(&path).display(),
                        current_chain.trim()
                    );
                    current_chain.clear();
                }
                current_chain.push_str(line);
                current_chain.push('\n');
            }
            let count = current_chain.matches(method).count();
            assert!(
                count <= 1,
                "Found {count} {method} calls on same element in {}: \n{}",
                path.strip_prefix(manifest_dir).unwrap_or(&path).display(),
                current_chain.trim()
            );
        }
    }

    #[test]
    fn test_no_duplicate_hover_chains_in_codebase() {
        assert_no_duplicate_modifiers(".hover(");
    }

    #[test]
    fn test_no_duplicate_active_chains_in_codebase() {
        assert_no_duplicate_modifiers(".active(");
    }

    #[test]
    fn test_all_referenced_svg_icons_exist() {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let src_dir = manifest_dir.join("src");
        let mut files = Vec::new();
        collect_source_files(&src_dir, &mut files);

        let mut missing = Vec::new();
        for path in files {
            let content = std::fs::read_to_string(&path).expect("read source file");
            for part in content.split('"') {
                if part.starts_with("icons/") && part.ends_with(".svg") {
                    let resolved = resolve_asset_path(part);
                    if EmbeddedAssets
                        .load(&resolved)
                        .expect("lookup works")
                        .is_none()
                    {
                        missing.push((
                            part.to_string(),
                            path.strip_prefix(manifest_dir)
                                .unwrap_or(&path)
                                .to_path_buf(),
                        ));
                    }
                }
            }
        }
        assert!(
            missing.is_empty(),
            "Found references to non-existent icons: {missing:?}"
        );
    }

    #[test]
    fn test_ibm_plex_fonts_are_embedded() {
        let expected_fonts = [
            "fonts/IBM Plex Mono/IBMPlexMono-Regular.ttf",
            "fonts/IBM Plex Mono/IBMPlexMono-Medium.ttf",
            "fonts/IBM Plex Mono/IBMPlexMono-SemiBold.ttf",
            "fonts/IBM Plex Mono/IBMPlexMono-Bold.ttf",
            "fonts/IBM Plex Mono/IBMPlexMono-Italic.ttf",
            "fonts/IBM Plex Sans/IBMPlexSans-VariableFont_wdth,wght.ttf",
        ];
        for font_path in expected_fonts {
            assert!(
                EmbeddedAssets
                    .load(font_path)
                    .expect("lookup font works")
                    .is_some(),
                "Font {font_path} should be embedded"
            );
        }
    }
}
