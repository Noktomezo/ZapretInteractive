use std::env;
use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

pub(crate) fn apply() -> Result<()> {
    apply_for_profile(None)
}

pub(crate) fn apply_release() -> Result<()> {
    apply_for_profile(Some("release"))
}

fn apply_for_profile(profile: Option<&str>) -> Result<()> {
    println!("=== Checking GPUI Checkout & Source Patch ===");
    let patch_file = PathBuf::from("patches/gpui-render-effects.patch");
    if !patch_file.exists() {
        println!("Patch file not found at {:?}, skipping.", patch_file);
        return Ok(());
    }
    let patch_abs = fs::canonicalize(&patch_file)?;
    let patch_str = patch_abs.to_string_lossy().replace(r"\\?\", "");

    let mut candidate_dirs = Vec::new();
    if let Ok(cargo_home) = env::var("CARGO_HOME") {
        candidate_dirs.push(PathBuf::from(cargo_home).join("git").join("checkouts"));
    }
    if let Ok(userprofile) = env::var("USERPROFILE") {
        candidate_dirs.push(
            PathBuf::from(userprofile)
                .join(".cargo")
                .join("git")
                .join("checkouts"),
        );
    }
    candidate_dirs.push(PathBuf::from(
        r"D:\Scoop\persist\rustup-msvc\.cargo\git\checkouts",
    ));

    let mut patched_count = 0;
    for base in candidate_dirs {
        if !base.exists() {
            continue;
        }
        if let Ok(entries) = fs::read_dir(&base) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("zed-") {
                    let zed_dir = entry.path();
                    if let Ok(sub_entries) = fs::read_dir(&zed_dir) {
                        for sub in sub_entries.flatten() {
                            let checkout_path = sub.path();
                            if checkout_path.join("crates").join("gpui").exists() {
                                let check_reverse = Command::new("git")
                                    .args(["apply", "--check", "--reverse"])
                                    .arg(&patch_str)
                                    .current_dir(&checkout_path)
                                    .output();
                                if let Ok(out) = check_reverse
                                    && out.status.success()
                                {
                                    println!(
                                        "Patch already applied in {}",
                                        checkout_path.display()
                                    );
                                    patched_count += 1;
                                    continue;
                                }

                                let check_forward = Command::new("git")
                                    .args(["apply", "--check"])
                                    .arg(&patch_str)
                                    .current_dir(&checkout_path)
                                    .output();
                                if let Ok(out) = check_forward
                                    && out.status.success()
                                {
                                    println!("Applying patch to {}", checkout_path.display());
                                    let apply = Command::new("git")
                                        .args(["apply", "--whitespace=nowarn"])
                                        .arg(&patch_str)
                                        .current_dir(&checkout_path)
                                        .status();
                                    if let Ok(status) = apply
                                        && status.success()
                                    {
                                        println!(
                                            "Successfully patched GPUI in {}",
                                            checkout_path.display()
                                        );
                                        patched_count += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    if patched_count == 0 {
        println!("Note: No unpatched GPUI checkouts found or checkouts were already patched.");
    }
    refresh_artifacts(&patch_abs, profile)?;
    Ok(())
}

fn refresh_artifacts(patch: &Path, profile: Option<&str>) -> Result<()> {
    let target_dir = env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target"));
    let stamp = target_dir.join(stamp_file_name(profile));
    let patch_bytes = fs::read(patch)
        .with_context(|| format!("failed to read GPUI patch at {}", patch.display()))?;
    let fingerprint = fingerprint(&patch_bytes);
    if fs::read_to_string(&stamp).is_ok_and(|stored| stored.trim() == fingerprint) {
        return Ok(());
    }

    println!(
        "GPUI patch changed; removing stale GPUI artifacts from {}",
        target_dir.display()
    );
    let mut clean = Command::new("cargo");
    clean.args(["clean", "-p", "gpui", "-p", "gpui_windows"]);
    if let Some(profile) = profile {
        clean.args(["--profile", profile]);
    }
    let status = clean
        .env("CARGO_TARGET_DIR", &target_dir)
        .status()
        .context("failed to run cargo clean for patched GPUI packages")?;
    if !status.success() {
        bail!("cargo clean failed for patched GPUI packages");
    }

    fs::create_dir_all(&target_dir)
        .with_context(|| format!("failed to create {}", target_dir.display()))?;
    fs::write(&stamp, fingerprint)
        .with_context(|| format!("failed to write {}", stamp.display()))?;
    Ok(())
}

fn fingerprint(bytes: &[u8]) -> String {
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn stamp_file_name(profile: Option<&str>) -> String {
    match profile {
        Some(profile) => format!(".gpui-patch-fingerprint-{profile}"),
        None => ".gpui-patch-fingerprint".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::{fingerprint, stamp_file_name};

    #[test]
    fn fingerprint_changes_with_patch_contents() {
        assert_eq!(fingerprint(b"same"), fingerprint(b"same"));
        assert_ne!(fingerprint(b"old"), fingerprint(b"new"));
    }

    #[test]
    fn release_uses_an_independent_artifact_stamp() {
        assert_eq!(stamp_file_name(None), ".gpui-patch-fingerprint");
        assert_eq!(
            stamp_file_name(Some("release")),
            ".gpui-patch-fingerprint-release"
        );
    }
}
