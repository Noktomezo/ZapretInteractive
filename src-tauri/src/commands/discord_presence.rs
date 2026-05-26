use crate::commands::config::DiscordPresenceActivityType;
use serde::Serialize;
use std::sync::LazyLock;
use tokio::sync::Mutex;

#[cfg(windows)]
use tokio::io::{AsyncReadExt, AsyncWriteExt};
#[cfg(windows)]
use tokio::net::windows::named_pipe::{ClientOptions, NamedPipeClient};

const DISCORD_CLIENT_ID: &str = "1495773045904769255";
const DISCORD_GITHUB_URL: &str = "https://github.com/Noktomezo/ZapretInteractive";

#[derive(Default)]
struct DiscordPresenceState {
    #[cfg(windows)]
    client: Option<NamedPipeClient>,
    last_key: Option<String>,
}

static DISCORD_PRESENCE_STATE: LazyLock<Mutex<DiscordPresenceState>> =
    LazyLock::new(|| Mutex::new(DiscordPresenceState::default()));

#[derive(Serialize)]
struct HandshakePayload {
    v: u32,
    #[serde(rename = "client_id")]
    client_id: &'static str,
}

#[derive(Serialize)]
struct SetActivityPayload {
    cmd: &'static str,
    args: SetActivityArgs,
    nonce: String,
}

#[derive(Serialize)]
struct SetActivityArgs {
    pid: u32,
    activity: Option<Activity>,
}

#[derive(Serialize)]
struct Activity {
    #[serde(rename = "type")]
    activity_type: u8,
    details: String,
    state: String,
    buttons: Vec<Button>,
}

#[derive(Serialize)]
struct Button {
    label: String,
    url: String,
}

fn map_activity_type(activity_type: DiscordPresenceActivityType) -> u8 {
    match activity_type {
        DiscordPresenceActivityType::Playing => 0,
        DiscordPresenceActivityType::Listening => 2,
        DiscordPresenceActivityType::Watching => 3,
        DiscordPresenceActivityType::Competing => 5,
    }
}

#[cfg(windows)]
async fn send_packet(
    client: &mut NamedPipeClient,
    opcode: u32,
    payload: &str,
) -> std::io::Result<()> {
    let bytes = payload.as_bytes();
    let len = bytes.len() as u32;
    client.write_all(&opcode.to_le_bytes()).await?;
    client.write_all(&len.to_le_bytes()).await?;
    client.write_all(bytes).await?;
    client.flush().await?;
    Ok(())
}

#[cfg(windows)]
async fn read_packet(client: &mut NamedPipeClient) -> std::io::Result<(u32, String)> {
    let mut header = [0u8; 8];
    client.read_exact(&mut header).await?;

    let opcode = u32::from_le_bytes([header[0], header[1], header[2], header[3]]);
    let len = u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as usize;

    let mut buffer = vec![0u8; len];
    client.read_exact(&mut buffer).await?;

    let payload = String::from_utf8(buffer)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    Ok((opcode, payload))
}

#[cfg(windows)]
async fn connect_to_discord_ipc() -> std::io::Result<NamedPipeClient> {
    for i in 0..10 {
        let pipe_path = format!(r"\\.\pipe\discord-ipc-{}", i);
        match ClientOptions::new().open(&pipe_path) {
            Ok(client) => return Ok(client),
            Err(e) if e.raw_os_error() == Some(231) => {
                // ERROR_PIPE_BUSY
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                if let Ok(client) = ClientOptions::new().open(&pipe_path) {
                    return Ok(client);
                }
            }
            Err(_) => continue,
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "No Discord IPC pipe found",
    ))
}

#[cfg(windows)]
async fn reconnect_client(state: &mut DiscordPresenceState) -> Result<(), String> {
    state.client = None;

    let mut client = connect_to_discord_ipc().await.map_err(|e| e.to_string())?;

    let handshake = HandshakePayload {
        v: 1,
        client_id: DISCORD_CLIENT_ID,
    };
    let handshake_str = serde_json::to_string(&handshake).map_err(|e| e.to_string())?;

    tokio::time::timeout(
        std::time::Duration::from_secs(2),
        send_packet(&mut client, 0, &handshake_str),
    )
    .await
    .map_err(|_| "Handshake write timeout".to_string())?
    .map_err(|e| format!("Failed to send handshake: {e}"))?;

    let (_opcode, _response) =
        tokio::time::timeout(std::time::Duration::from_secs(2), read_packet(&mut client))
            .await
            .map_err(|_| "Handshake read timeout".to_string())?
            .map_err(|e| format!("Failed to read handshake response: {e}"))?;

    state.client = Some(client);
    Ok(())
}

#[cfg(windows)]
async fn clear_presence(state: &mut DiscordPresenceState) {
    state.last_key = None;
    if let Some(mut client) = state.client.take() {
        let payload = SetActivityPayload {
            cmd: "SET_ACTIVITY",
            args: SetActivityArgs {
                pid: std::process::id(),
                activity: None,
            },
            nonce: uuid::Uuid::new_v4().to_string(),
        };
        if let Ok(payload_str) = serde_json::to_string(&payload) {
            let _ = tokio::time::timeout(
                std::time::Duration::from_secs(1),
                send_packet(&mut client, 1, &payload_str),
            )
            .await;
        }
    }
}

#[cfg(windows)]
#[tauri::command]
pub async fn sync_discord_presence(
    enabled: bool,
    details: String,
    state: String,
    activity_type: DiscordPresenceActivityType,
) -> Result<bool, String> {
    let mut presence_state = DISCORD_PRESENCE_STATE.lock().await;

    if !enabled {
        clear_presence(&mut presence_state).await;
        return Ok(true);
    }

    let next_key = format!("{activity_type:?}\u{0}{details}\u{0}{state}");

    if presence_state.client.is_none()
        && let Err(error) = reconnect_client(&mut presence_state).await
    {
        eprintln!("Discord presence: initial reconnect failed: {error}");
        return Ok(false);
    }

    let payload = SetActivityPayload {
        cmd: "SET_ACTIVITY",
        args: SetActivityArgs {
            pid: std::process::id(),
            activity: Some(Activity {
                activity_type: map_activity_type(activity_type),
                details: details.clone(),
                state: state.clone(),
                buttons: vec![Button {
                    label: "Доступ в интернет".to_string(),
                    url: DISCORD_GITHUB_URL.to_string(),
                }],
            }),
        },
        nonce: uuid::Uuid::new_v4().to_string(),
    };

    let payload_str = serde_json::to_string(&payload).map_err(|e| e.to_string())?;

    let client = presence_state
        .client
        .as_mut()
        .ok_or("Client not initialized")?;

    let update_result = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        send_packet(client, 1, &payload_str),
    )
    .await;

    match update_result {
        Ok(Ok(())) => {
            presence_state.last_key = Some(next_key);
            Ok(true)
        }
        _ => {
            clear_presence(&mut presence_state).await;

            if let Err(reconnect_error) = reconnect_client(&mut presence_state).await {
                eprintln!("Discord presence: retry reconnect failed: {reconnect_error}");
                return Ok(false);
            }

            let client = presence_state
                .client
                .as_mut()
                .ok_or("Client not initialized after reconnect")?;

            let retry_result = tokio::time::timeout(
                std::time::Duration::from_secs(2),
                send_packet(client, 1, &payload_str),
            )
            .await;

            match retry_result {
                Ok(Ok(())) => {
                    presence_state.last_key = Some(next_key);
                    Ok(true)
                }
                _ => {
                    clear_presence(&mut presence_state).await;
                    eprintln!("Discord presence: activity update retry failed");
                    Ok(false)
                }
            }
        }
    }
}

#[cfg(not(windows))]
#[tauri::command]
pub async fn sync_discord_presence(
    _enabled: bool,
    _details: String,
    _state: String,
    _activity_type: DiscordPresenceActivityType,
) -> Result<bool, String> {
    Ok(false)
}
