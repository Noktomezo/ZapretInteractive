use discord_rich_presence::{DiscordIpc, DiscordIpcClient, activity};
use std::sync::{LazyLock, Mutex};

const DISCORD_CLIENT_ID: &str = "1495773045904769255";

#[derive(Default)]
struct DiscordPresenceState {
    client: Option<DiscordIpcClient>,
    last_key: Option<String>,
}

static DISCORD_PRESENCE_STATE: LazyLock<Mutex<DiscordPresenceState>> =
    LazyLock::new(|| Mutex::new(DiscordPresenceState::default()));

fn discord_error_to_string<E: std::fmt::Display>(error: E) -> String {
    error.to_string()
}

fn reconnect_client(state: &mut DiscordPresenceState) -> Result<(), String> {
    let mut client = DiscordIpcClient::new(DISCORD_CLIENT_ID);
    client.connect().map_err(discord_error_to_string)?;
    state.client = Some(client);
    Ok(())
}

fn clear_presence(state: &mut DiscordPresenceState) {
    state.last_key = None;
    if let Some(mut client) = state.client.take() {
        let _ = client.clear_activity();
        let _ = client.close();
    }
}

fn log_reconnect_failure(context: &str, error: &str) {
    eprintln!("Discord presence: {context}: {error}");
}

#[tauri::command]
pub fn sync_discord_presence(
    enabled: bool,
    details: String,
    state: String,
) -> Result<bool, String> {
    let mut presence_state = DISCORD_PRESENCE_STATE.lock().map_err(|e| e.to_string())?;

    if !enabled {
        clear_presence(&mut presence_state);
        return Ok(true);
    }

    let next_key = format!("{details}\u{0}{state}");
    if presence_state.last_key.as_deref() == Some(next_key.as_str()) {
        return Ok(true);
    }

    if presence_state.client.is_none()
        && let Err(error) = reconnect_client(&mut presence_state)
    {
        log_reconnect_failure("initial reconnect failed", &error);
        return Ok(false);
    }

    let activity = activity::Activity::new()
        .details(details.clone())
        .state(state.clone());

    let update_result = presence_state
        .client
        .as_mut()
        .ok_or_else(|| "Discord client was not initialized".to_string())
        .and_then(|client| {
            client
                .set_activity(activity)
                .map_err(discord_error_to_string)
        });

    match update_result {
        Ok(()) => {
            presence_state.last_key = Some(next_key);
            Ok(true)
        }
        Err(error) => {
            clear_presence(&mut presence_state);
            if let Err(reconnect_error) = reconnect_client(&mut presence_state) {
                log_reconnect_failure(
                    "retry reconnect failed after activity update error",
                    &reconnect_error,
                );
                return Ok(false);
            }

            let retry_activity = activity::Activity::new().details(details).state(state);
            if let Some(client) = presence_state.client.as_mut()
                && client.set_activity(retry_activity).is_ok()
            {
                presence_state.last_key = Some(next_key);
                return Ok(true);
            }

            eprintln!("Discord presence: activity update failed: {error}");
            Ok(false)
        }
    }
}
