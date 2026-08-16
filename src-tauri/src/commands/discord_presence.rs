use crate::commands::config::DiscordPresenceActivityType;
use discord_rich_presence::{DiscordIpc, DiscordIpcClient, activity};
use std::sync::LazyLock;
use tokio::sync::Mutex;

const DISCORD_CLIENT_ID: &str = "1495773045904769255";
const DISCORD_GITHUB_URL: &str = "https://github.com/Noktomezo/ZapretInteractive";

#[derive(Default)]
struct DiscordPresenceState {
    #[cfg(windows)]
    client: Option<DiscordIpcClient>,
    last_key: Option<String>,
}

static DISCORD_PRESENCE_STATE: LazyLock<Mutex<DiscordPresenceState>> =
    LazyLock::new(|| Mutex::new(DiscordPresenceState::default()));

#[cfg(windows)]
async fn clear_presence(state: &mut DiscordPresenceState) {
    state.last_key = None;
    if let Some(mut client) = state.client.take() {
        let _ = client.clear_activity();
        let _ = client.close();
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

    if presence_state.last_key.as_ref() == Some(&next_key) {
        return Ok(true);
    }

    if presence_state.client.is_none() {
        let mut new_client = DiscordIpcClient::new(DISCORD_CLIENT_ID);
        if let Err(err) = new_client.connect() {
            eprintln!("Discord presence: failed to connect: {err}");
            return Ok(false);
        }
        presence_state.client = Some(new_client);
    }

    let client = presence_state
        .client
        .as_mut()
        .ok_or("Client not initialized")?;

    let buttons = vec![activity::Button::new(
        "Доступ в интернет",
        DISCORD_GITHUB_URL,
    )];
    let payload = activity::Activity::new()
        .details(&details)
        .state(&state)
        .buttons(buttons);

    match client.set_activity(payload) {
        Ok(_) => {
            presence_state.last_key = Some(next_key);
            Ok(true)
        }
        Err(e) => {
            eprintln!("Discord presence: failed to set activity: {e}");
            // Reset client on error so next attempt tries reconnecting
            if let Some(mut client_to_close) = presence_state.client.take() {
                let _ = client_to_close.close();
            }
            presence_state.last_key = None;
            Ok(false)
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
