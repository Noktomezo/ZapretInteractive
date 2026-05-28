use crate::commands::config::DiscordPresenceActivityType;
use discord_presence::Client as DiscordClient;
use discord_presence::models::rich_presence::{Activity, ActivityButton};
use std::sync::LazyLock;
use tokio::sync::Mutex;

const DISCORD_CLIENT_ID: u64 = 1495773045904769255;
const DISCORD_GITHUB_URL: &str = "https://github.com/Noktomezo/ZapretInteractive";

#[derive(Default)]
struct DiscordPresenceState {
    #[cfg(windows)]
    client: Option<DiscordClient>,
    last_key: Option<String>,
}

static DISCORD_PRESENCE_STATE: LazyLock<Mutex<DiscordPresenceState>> =
    LazyLock::new(|| Mutex::new(DiscordPresenceState::default()));

fn map_activity_type(
    activity_type: DiscordPresenceActivityType,
) -> discord_presence::models::rich_presence::ActivityType {
    use discord_presence::models::rich_presence::ActivityType;
    match activity_type {
        DiscordPresenceActivityType::Playing => ActivityType::Playing,
        DiscordPresenceActivityType::Listening => ActivityType::Listening,
        DiscordPresenceActivityType::Watching => ActivityType::Watching,
        DiscordPresenceActivityType::Competing => ActivityType::Competing,
    }
}

#[cfg(windows)]
async fn clear_presence(state: &mut DiscordPresenceState) {
    state.last_key = None;
    if let Some(mut client) = state.client.take() {
        let _ = client.clear_activity();
        // Dropping the client automatically halts the background thread and closes the connection
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
        let mut client = DiscordClient::new(DISCORD_CLIENT_ID);
        // Start the background thread which manages automatic reconnect retry loops and heartbeats
        client.start();
        presence_state.client = Some(client);
    }

    let client = presence_state
        .client
        .as_mut()
        .ok_or("Client not initialized")?;

    let details_clone = details.clone();
    let state_clone = state.clone();

    let update_result = client.set_activity(|_| Activity {
        details: Some(details_clone),
        state: Some(state_clone),
        activity_type: Some(map_activity_type(activity_type)),
        buttons: vec![ActivityButton {
            label: Some("Доступ в интернет".to_string()),
            url: Some(DISCORD_GITHUB_URL.to_string()),
        }],
        instance: Some(true),
        ..Default::default()
    });

    match update_result {
        Ok(_) => {
            presence_state.last_key = Some(next_key);
            Ok(true)
        }
        Err(e) => {
            eprintln!("Discord presence: failed to set activity: {e}");
            // Return Ok(false) so frontend can retry since connection might still be establishing in the background thread
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
