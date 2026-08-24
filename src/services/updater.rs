use anyhow::{Context, Result, bail};
use futures_util::StreamExt;
use reqwest::Client;
use reqwest::header::USER_AGENT;
use semver::Version;
use serde::Deserialize;
use std::{path::PathBuf, time::Duration};
use tokio::io::AsyncWriteExt;

const LATEST_RELEASE_WEB: &str = "https://github.com/Noktomezo/ZapretInteractive/releases/latest";
const LATEST_RELEASE_API: &str =
    "https://api.github.com/repos/Noktomezo/ZapretInteractive/releases/latest";

#[derive(Clone, Debug, Deserialize)]
pub struct GitHubReleaseAsset {
    pub name: String,
    pub browser_download_url: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GitHubReleaseResponse {
    pub tag_name: String,
    pub html_url: String,
    #[serde(default)]
    pub assets: Vec<GitHubReleaseAsset>,
}

#[derive(Clone, Debug)]
pub struct AppUpdateInfo {
    pub new_version: String,
    pub release_url: String,
    pub download_url: Option<String>,
}

pub async fn check_app_update(
    client: &Client,
    current_version_str: &str,
) -> Result<Option<AppUpdateInfo>> {
    let current_semver = Version::parse(current_version_str.trim_start_matches('v'))
        .with_context(|| format!("failed to parse current semver '{current_version_str}'"))?;

    // 1. Primary strategy: Check via web releases redirect (immune to GitHub API rate limits / 403)
    if let Ok(response) = client
        .get(LATEST_RELEASE_WEB)
        .header(
            USER_AGENT,
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36",
        )
        .send()
        .await
        && response.status().is_success()
    {
            let final_url = response.url().to_string();
            if let Some(tag_part) = final_url.split("/releases/tag/").nth(1) {
                let tag = tag_part.split('?').next().unwrap_or(tag_part).trim();
                let remote_tag = tag.trim_start_matches('v');
                if let Ok(remote_semver) = Version::parse(remote_tag) {
                    if remote_semver > current_semver {
                        let download_url = fetch_asset_from_api(client, tag)
                            .await
                            .unwrap_or_else(|| fallback_installer_url(tag));
                        return Ok(Some(AppUpdateInfo {
                            new_version: tag.to_string(),
                            release_url: final_url,
                            download_url: Some(download_url),
                        }));
                    } else {
                        return Ok(None);
                    }
                }
            }
    }

    // 2. Fallback strategy: Query the GitHub REST API
    let response = client
        .get(LATEST_RELEASE_API)
        .header(USER_AGENT, "ZapretInteractive-Updater")
        .send()
        .await
        .context("failed to query GitHub releases API")?;

    if response.status() == reqwest::StatusCode::FORBIDDEN {
        // Rate limit reached on GitHub REST API, safely treat as up-to-date
        return Ok(None);
    }

    if !response.status().is_success() {
        bail!("GitHub API returned HTTP {}", response.status());
    }

    let release: GitHubReleaseResponse = response.json().await?;
    let remote_tag = release.tag_name.trim().trim_start_matches('v');
    let remote_semver = match Version::parse(remote_tag) {
        Ok(v) => v,
        Err(_) => return Ok(None),
    };

    if remote_semver > current_semver {
        let download_url = installer_asset_url(release.assets);

        Ok(Some(AppUpdateInfo {
            new_version: release.tag_name,
            release_url: release.html_url,
            download_url,
        }))
    } else {
        Ok(None)
    }
}

async fn fetch_asset_from_api(client: &Client, tag: &str) -> Option<String> {
    let api_url =
        format!("https://api.github.com/repos/Noktomezo/ZapretInteractive/releases/tags/{tag}");
    let response = client
        .get(&api_url)
        .header(USER_AGENT, "ZapretInteractive-Updater")
        .send()
        .await
        .ok()?;

    if !response.status().is_success() {
        return None;
    }

    let release: GitHubReleaseResponse = response.json().await.ok()?;
    installer_asset_url(release.assets)
}

fn installer_asset_url(assets: Vec<GitHubReleaseAsset>) -> Option<String> {
    assets
        .into_iter()
        .find(|asset| {
            let name = asset.name.to_ascii_lowercase();
            name.ends_with("-installer.exe") || name.ends_with("-setup.exe")
        })
        .map(|asset| asset.browser_download_url)
}

fn fallback_installer_url(tag: &str) -> String {
    let version = tag.trim_start_matches('v');
    format!(
        "https://github.com/Noktomezo/ZapretInteractive/releases/download/{tag}/Zapret.Interactive_{version}_x64-installer.exe"
    )
}

pub async fn download_and_install_app_update<F, G>(
    client: &Client,
    download_url: &str,
    mut on_progress: F,
    mut on_restarting: G,
) -> Result<()>
where
    F: FnMut(f32) + Send + 'static,
    G: FnMut() + Send + 'static,
{
    let response = client
        .get(download_url)
        .header(USER_AGENT, "ZapretInteractive-Updater")
        .send()
        .await
        .context("failed to request update download")?;

    if !response.status().is_success() {
        bail!("HTTP {} downloading update", response.status());
    }

    let total_size = response.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;
    let mut executable_header = [0_u8; 2];
    let mut header_len = 0;

    let temp_dir = std::env::temp_dir();
    let installer_path: PathBuf = temp_dir.join(format!(
        "ZapretInteractive-setup-{}.exe",
        uuid::Uuid::new_v4().simple()
    ));
    let mut file = tokio::fs::File::create(&installer_path)
        .await
        .with_context(|| {
            format!(
                "failed to create temp installer at {}",
                installer_path.display()
            )
        })?;

    let mut stream = response.bytes_stream();

    while let Some(chunk_res) = stream.next().await {
        let chunk = chunk_res.context("error reading download stream")?;
        let header_bytes = (executable_header.len() - header_len).min(chunk.len());
        executable_header[header_len..header_len + header_bytes]
            .copy_from_slice(&chunk[..header_bytes]);
        header_len += header_bytes;
        file.write_all(&chunk)
            .await
            .context("error writing update chunk to file")?;
        downloaded += chunk.len() as u64;
        if total_size > 0 {
            let progress = (downloaded as f32 / total_size as f32).clamp(0.0, 1.0);
            on_progress(progress);
        }
    }

    if total_size > 0 && downloaded != total_size {
        bail!("update download ended at {downloaded} of {total_size} bytes");
    }
    if header_len != executable_header.len() || executable_header != *b"MZ" {
        bail!("downloaded update is not a Windows executable");
    }

    file.flush().await.context("error flushing update file")?;
    drop(file);

    on_restarting();
    tokio::time::sleep(Duration::from_millis(300)).await;

    #[cfg(windows)]
    {
        duct::cmd(&installer_path, ["/P", "/R"])
            .start()
            .with_context(|| {
                format!("failed to launch installer at {}", installer_path.display())
            })?;
    }
    #[cfg(not(windows))]
    drop(installer_path);

    std::process::exit(0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semver_comparison_logic() {
        let current = Version::parse("1.6.2").unwrap();
        let newer = Version::parse("1.6.3").unwrap();
        let major_newer = Version::parse("2.0.0").unwrap();
        let older = Version::parse("1.6.1").unwrap();
        let same = Version::parse("1.6.2").unwrap();

        assert!(newer > current);
        assert!(major_newer > current);
        assert!(older < current);
        assert!(same == current);

        let v_tag = "v1.6.3".trim_start_matches('v');
        let parsed_v = Version::parse(v_tag).unwrap();
        assert!(parsed_v > current);
    }

    #[test]
    fn test_release_response_deserialization() {
        let json_payload = r#"{
            "tag_name": "v2.0.1",
            "html_url": "https://github.com/Noktomezo/ZapretInteractive/releases/tag/v2.0.1",
            "assets": [
                {
                    "name": "Zapret.Interactive_2.0.1_x64-portable.zip",
                    "browser_download_url": "https://example.invalid/portable.zip"
                },
                {
                    "name": "Zapret.Interactive_2.0.1_x64-installer.exe",
                    "browser_download_url": "https://example.invalid/installer.exe"
                }
            ]
        }"#;

        let release: GitHubReleaseResponse = serde_json::from_str(json_payload).unwrap();
        assert_eq!(release.tag_name, "v2.0.1");
        assert_eq!(release.assets.len(), 2);
        assert_eq!(
            installer_asset_url(release.assets).as_deref(),
            Some("https://example.invalid/installer.exe")
        );
    }
}
