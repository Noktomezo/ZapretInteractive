use std::env;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use chrono::Utc;
use serde::Serialize;
use zip::write::SimpleFileOptions;

mod gpui_patch;

const MAIN_PACKAGE_CARGO_TOML: &str = include_str!("../../../Cargo.toml");

#[derive(Serialize)]
struct LatestManifest {
    version: String,
    notes: String,
    pub_date: String,
    platforms: ManifestPlatforms,
}

#[derive(Serialize)]
struct ManifestPlatforms {
    #[serde(rename = "windows-x86_64")]
    windows_x86_64: PlatformTarget,
}

#[derive(Serialize)]
struct PlatformTarget {
    signature: String,
    url: String,
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    let task = args.first().map(|s| s.as_str()).unwrap_or("help");

    match task {
        "build" | "dist" | "bundle" => {
            build_release()?;
            if find_makensis().is_some() {
                build_nsis_bundle()?;
            } else {
                println!("Note: NSIS not found, skipping NSIS installer packaging.");
            }
            build_portable_bundle()?;
        }
        "manifest" => generate_manifest(&args)?,
        "patch-gpui" => gpui_patch::apply()?,
        "dev-icon" | "make-dev-icon" => make_dev_icon()?,
        "help" | _ => print_help(),
    }
    Ok(())
}

fn print_help() {
    println!("Zapret Interactive XTask Tooling");
    println!("Usage: cargo xtask <task>");
    println!();
    println!("Available tasks:");
    println!("  build       - Compile release binary, NSIS installer and portable ZIP bundle");
    println!("  dist        - Alias for build");
    println!("  bundle      - Alias for build");
    println!("  manifest    - Generate latest.json for in-app updater compatibility");
    println!("  patch-gpui  - Apply custom D3D11 / shader patch to GPUI checkout in cargo cache");
}

fn get_app_version() -> Result<String> {
    for line in MAIN_PACKAGE_CARGO_TOML.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("version = \"") {
            if let Some(version) = rest.strip_suffix('"') {
                return Ok(version.to_string());
            }
        }
    }
    bail!("Could not parse version from Cargo.toml");
}

fn find_makensis() -> Option<PathBuf> {
    let standard_paths = [
        PathBuf::from(r"C:\Program Files (x86)\NSIS\makensis.exe"),
        PathBuf::from(r"C:\Program Files\NSIS\makensis.exe"),
    ];

    for path in &standard_paths {
        if path.is_file() {
            return Some(path.clone());
        }
    }

    if Command::new("makensis")
        .arg("/VERSION")
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
    {
        return Some(PathBuf::from("makensis"));
    }

    None
}

fn build_release() -> Result<()> {
    println!("=== Building Release Binary ===");
    gpui_patch::apply_release()?;
    let status = Command::new("cargo")
        .args([
            "build",
            "--release",
            "--package",
            "zapret-interactive",
            "--bin",
            "ZapretInteractive",
        ])
        .status()
        .context("Failed to execute cargo build")?;

    if !status.success() {
        bail!("Release build failed with exit code {:?}", status.code());
    }

    let compiled_exe = PathBuf::from("target/release/ZapretInteractive.exe");
    if !compiled_exe.is_file() {
        bail!("Built binary not found at {}", compiled_exe.display());
    }

    let shipping_exe = PathBuf::from("target/release/Zapret Interactive.exe");
    if shipping_exe.is_file() {
        fs::remove_file(&shipping_exe).with_context(|| {
            format!(
                "Failed to replace shipping executable {}",
                shipping_exe.display()
            )
        })?;
    }
    fs::rename(&compiled_exe, &shipping_exe).with_context(|| {
        format!(
            "Failed to rename {} to {}",
            compiled_exe.display(),
            shipping_exe.display()
        )
    })?;
    sync_release_resources()?;

    println!("Release binary ready at {}", shipping_exe.display());
    Ok(())
}

fn sync_release_resources() -> Result<()> {
    let source = Path::new("thirdparty");
    let destination = Path::new("target/release/resources");
    if destination.exists() {
        fs::remove_dir_all(destination)
            .with_context(|| format!("Failed to clear {}", destination.display()))?;
    }

    for entry in walkdir::WalkDir::new(source) {
        let entry = entry?;
        let relative = entry.path().strip_prefix(source)?;
        let target = destination.join(relative);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target)?;
        } else if entry.file_type().is_file() {
            fs::copy(entry.path(), &target)
                .with_context(|| format!("Failed to copy resources to {}", target.display()))?;
        }
    }

    println!("Release resources ready at {}", destination.display());
    Ok(())
}

fn convert_png_to_bmp(
    src_png: &Path,
    dst_bmp: &Path,
    target_width: u32,
    target_height: u32,
) -> Result<()> {
    let img = image::open(src_png)
        .with_context(|| format!("Failed to open image {}", src_png.display()))?;

    let img = img.resize(
        target_width,
        target_height,
        image::imageops::FilterType::Lanczos3,
    );

    let mut bg =
        image::RgbImage::from_pixel(target_width, target_height, image::Rgb([255, 255, 255]));

    let x_offset = (target_width.saturating_sub(img.width())) / 2;
    let y_offset = (target_height.saturating_sub(img.height())) / 2;

    let rgba_img = img.to_rgba8();
    for (x, y, pixel) in rgba_img.enumerate_pixels() {
        let alpha = pixel[3] as f32 / 255.0;
        let bg_pixel = bg.get_pixel_mut(x + x_offset, y + y_offset);
        let r = ((1.0 - alpha) * 255.0 + alpha * (pixel[0] as f32)) as u8;
        let g = ((1.0 - alpha) * 255.0 + alpha * (pixel[1] as f32)) as u8;
        let b = ((1.0 - alpha) * 255.0 + alpha * (pixel[2] as f32)) as u8;
        *bg_pixel = image::Rgb([r, g, b]);
    }

    bg.save_with_format(dst_bmp, image::ImageFormat::Bmp)
        .with_context(|| format!("Failed to save BMP to {}", dst_bmp.display()))?;

    Ok(())
}

fn prepare_nsis_bitmaps(
    root_dir: &Path,
    bundle_nsis_dir: &Path,
) -> Result<(Option<PathBuf>, Option<PathBuf>)> {
    let header_png = root_dir.join("assets/app-installer-header.png");
    let sidebar_png = root_dir.join("assets/app-installer-sidebar.png");

    let header_bmp = if header_png.is_file() {
        let dst = bundle_nsis_dir.join("header.bmp");
        convert_png_to_bmp(&header_png, &dst, 150, 57)?;
        println!("Converted header to BMP: {}", dst.display());
        Some(dst)
    } else {
        None
    };

    let sidebar_bmp = if sidebar_png.is_file() {
        let dst = bundle_nsis_dir.join("sidebar.bmp");
        convert_png_to_bmp(&sidebar_png, &dst, 164, 314)?;
        println!("Converted sidebar to BMP: {}", dst.display());
        Some(dst)
    } else {
        None
    };

    Ok((header_bmp, sidebar_bmp))
}

fn build_nsis_bundle() -> Result<()> {
    let version = get_app_version()?;
    println!("=== Packaging NSIS Installer v{version} ===");

    let makensis = find_makensis().context(
        "makensis executable not found. Please install NSIS (e.g. `choco install nsis` or `scoop install nsis`).",
    )?;

    let root_dir = env::current_dir().context("Failed to get current working directory")?;
    let nsi_script = root_dir.join("tooling/packaging/nsis/installer.nsi");
    if !nsi_script.is_file() {
        bail!("NSIS script not found at {}", nsi_script.display());
    }

    let bundle_nsis_dir = root_dir.join("target/release/bundle/nsis");
    fs::create_dir_all(&bundle_nsis_dir)
        .context("Failed to create target/release/bundle/nsis directory")?;

    let (header_bmp, sidebar_bmp) = prepare_nsis_bitmaps(&root_dir, &bundle_nsis_dir)?;

    let setup_exe_name = format!("Zapret Interactive_{version}_x64-setup.exe");
    let bundle_setup_exe = bundle_nsis_dir.join(&setup_exe_name);

    let legacy_exe = root_dir.join("target/release").join(&setup_exe_name);
    if legacy_exe.is_file() {
        fs::remove_file(&legacy_exe).with_context(|| {
            format!("Failed to remove legacy installer {}", legacy_exe.display())
        })?;
    }

    let mut cmd = Command::new(&makensis);
    cmd.arg("/INPUTCHARSET")
        .arg("UTF8")
        .arg(format!("/DPRODUCT_VERSION={version}"))
        .arg(format!("/DOUTFILE={}", bundle_setup_exe.display()));

    if let Some(ref header) = header_bmp {
        cmd.arg(format!("/DHEADER_BMP={}", header.display()));
    }
    if let Some(ref sidebar) = sidebar_bmp {
        cmd.arg(format!("/DSIDEBAR_BMP={}", sidebar.display()));
    }

    cmd.arg(&nsi_script);

    let nsis_status = cmd
        .status()
        .with_context(|| format!("Failed to run {}", makensis.display()))?;

    if !nsis_status.success() {
        bail!("makensis failed with exit code {:?}", nsis_status.code());
    }

    if !bundle_setup_exe.is_file() {
        bail!(
            "Expected NSIS output {} not found",
            bundle_setup_exe.display()
        );
    }

    println!("Successfully created NSIS Installer:");
    println!("  Bundle: {}", bundle_setup_exe.display());

    Ok(())
}

fn build_portable_bundle() -> Result<()> {
    let version = get_app_version()?;
    println!("=== Packaging Portable ZIP Bundle v{version} ===");

    let root_dir = env::current_dir().context("Failed to get current working directory")?;
    let bundle_portable_dir = root_dir.join("target/release/bundle/portable");
    fs::create_dir_all(&bundle_portable_dir)
        .context("Failed to create target/release/bundle/portable directory")?;

    let zip_name = format!("Zapret Interactive_{version}_x64-portable.zip");
    let bundle_zip_path = bundle_portable_dir.join(&zip_name);

    let legacy_zip = root_dir.join("target/release").join(&zip_name);
    if legacy_zip.is_file() {
        fs::remove_file(&legacy_zip)
            .with_context(|| format!("Failed to remove legacy archive {}", legacy_zip.display()))?;
    }

    let zip_file = File::create(&bundle_zip_path)?;
    let mut zip = zip::ZipWriter::new(zip_file);
    let file_options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    let exe_path = root_dir.join("target/release/Zapret Interactive.exe");
    if !exe_path.is_file() {
        bail!("Release binary not found at {}", exe_path.display());
    }
    zip.start_file("Zapret Interactive.exe", file_options)?;
    let mut reader = BufReader::new(File::open(&exe_path)?);
    std::io::copy(&mut reader, &mut zip)?;

    let thirdparty_dir = root_dir.join("thirdparty");
    if thirdparty_dir.is_dir() {
        for entry in walkdir::WalkDir::new(&thirdparty_dir) {
            let entry = entry?;
            let path = entry.path();
            if let Ok(rel_path) = path.strip_prefix(&thirdparty_dir) {
                let rel_str = rel_path.to_string_lossy().replace('\\', "/");
                if rel_str.is_empty() {
                    continue;
                }
                if path.is_file() {
                    zip.start_file(format!("resources/{rel_str}"), file_options)?;
                    let mut file_reader = BufReader::new(File::open(path)?);
                    std::io::copy(&mut file_reader, &mut zip)?;
                } else if path.is_dir() {
                    zip.add_directory(format!("resources/{rel_str}"), file_options)?;
                }
            }
        }
    }

    zip.finish()?;

    println!("Successfully created Portable bundle:");
    println!("  Bundle: {}", bundle_zip_path.display());

    Ok(())
}

fn generate_manifest(args: &[String]) -> Result<()> {
    let version = get_app_version()?;
    let tag = format!("v{version}");
    let zip_name = format!("Zapret Interactive_{version}_x64-portable.zip");
    let encoded_zip_name = zip_name.replace(' ', "%20");
    let url = format!(
        "https://github.com/Noktomezo/ZapretInteractive/releases/download/{tag}/{encoded_zip_name}"
    );

    let signature = if let Some(sig_idx) = args.iter().position(|a| a == "--sig") {
        args.get(sig_idx + 1)
            .cloned()
            .context("--sig requires a signature value or file path")?
    } else if let Ok(sig_file) =
        fs::read_to_string(format!("target/release/bundle/portable/{zip_name}.sig"))
            .or_else(|_| fs::read_to_string(format!("target/release/bundle/nsis/{zip_name}.sig")))
    {
        sig_file.trim().to_string()
    } else if let Ok(sig_env) = env::var("MINISIGN_SIGNATURE") {
        sig_env
    } else {
        String::new()
    };

    let manifest = LatestManifest {
        version: tag,
        notes: format!("Zapret Interactive {version} Release"),
        pub_date: Utc::now().to_rfc3339(),
        platforms: ManifestPlatforms {
            windows_x86_64: PlatformTarget { signature, url },
        },
    };

    let json_content = serde_json::to_string_pretty(&manifest)?;

    let paths = [
        PathBuf::from("target/release/bundle/latest.json"),
        PathBuf::from("target/release/bundle/portable/latest.json"),
        PathBuf::from("target/release/bundle/nsis/latest.json"),
    ];

    for path in &paths {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create manifest directory {}", parent.display())
            })?;
        }
        fs::write(path, &json_content)?;
        println!("Generated manifest at {}", path.display());
    }

    Ok(())
}

fn make_dev_icon() -> Result<()> {
    let png_path = Path::new("assets/app-dev.png");
    if !png_path.is_file() {
        bail!("assets/app-dev.png not found in repository");
    }
    let img_bytes = fs::read(png_path)?;
    let base_image = image::load_from_memory_with_format(&img_bytes, image::ImageFormat::Png)
        .context("Failed to decode assets/app-dev.png")?;

    let sizes = [16u32, 20, 24, 30, 32, 40, 48, 64, 72, 96, 128, 256];
    let mut png_frames = Vec::new();
    for &sz in &sizes {
        let resized = base_image.resize_exact(sz, sz, image::imageops::FilterType::Lanczos3);
        let mut buf = std::io::Cursor::new(Vec::new());
        resized.write_to(&mut buf, image::ImageFormat::Png)?;
        png_frames.push((sz, buf.into_inner()));
    }

    let mut ico_data = Vec::new();
    // ICONDIR header: 6 bytes
    ico_data.extend_from_slice(&0u16.to_le_bytes()); // idReserved
    ico_data.extend_from_slice(&1u16.to_le_bytes()); // idType = 1 (icon)
    ico_data.extend_from_slice(&(png_frames.len() as u16).to_le_bytes()); // idCount

    let header_size = 6 + 16 * png_frames.len();
    let mut current_offset = header_size as u32;

    // Directory entries
    for (sz, frame_data) in &png_frames {
        let b_width = if *sz >= 256 { 0u8 } else { *sz as u8 };
        let b_height = if *sz >= 256 { 0u8 } else { *sz as u8 };
        ico_data.push(b_width);
        ico_data.push(b_height);
        ico_data.push(0); // bColorCount
        ico_data.push(0); // bReserved
        ico_data.extend_from_slice(&1u16.to_le_bytes()); // wPlanes
        ico_data.extend_from_slice(&32u16.to_le_bytes()); // wBitCount
        ico_data.extend_from_slice(&(frame_data.len() as u32).to_le_bytes()); // dwBytesInRes
        ico_data.extend_from_slice(&current_offset.to_le_bytes()); // dwImageOffset

        current_offset += frame_data.len() as u32;
    }

    // Image data
    for (_, frame_data) in png_frames {
        ico_data.extend_from_slice(&frame_data);
    }

    fs::write("assets/app-dev.ico", ico_data)?;
    println!(
        "Successfully generated multi-resolution assets/app-dev.ico with {} sizes (16..256)",
        sizes.len()
    );
    Ok(())
}
