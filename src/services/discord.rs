use std::sync::{Arc, Mutex};

use anyhow::{Context as _, Result};
use discord_presence::Client;
use discord_presence::models::rich_presence::{Activity, ActivityButton, ActivityType};

use crate::domain::{AppConfig, DiscordActivity};

const CLIENT_ID: u64 = 1_495_773_045_904_769_255;
const PROJECT_URL: &str = "https://github.com/Noktomezo/ZapretInteractive";

#[derive(Default)]
pub struct DiscordPresence {
    client: Option<Client>,
    desired_activity: Arc<Mutex<Option<Activity>>>,
    last_key: Option<String>,
}

impl DiscordPresence {
    pub fn sync(&mut self, config: &AppConfig, connected: bool) -> Result<()> {
        if !config.discord_presence_enabled {
            *self
                .desired_activity
                .lock()
                .map_err(|_| anyhow::anyhow!("discord activity lock poisoned"))? = None;
            if Client::is_ready()
                && let Some(client) = self.client.as_mut()
            {
                client
                    .clear_activity()
                    .context("не удалось очистить Discord Presence")?;
            }
            self.last_key = None;
            return Ok(());
        }

        let details = if connected {
            "Обход активен"
        } else {
            "Готов к подключению"
        };
        let state = if connected {
            "Подключено"
        } else {
            "Отключено"
        };
        let key = format!(
            "{:?}:{details}:{state}",
            config.discord_presence_activity_type
        );
        if self.last_key.as_deref() == Some(&key) {
            return Ok(());
        }

        let activity_type = match config.discord_presence_activity_type {
            DiscordActivity::Playing => ActivityType::Playing,
            DiscordActivity::Listening => ActivityType::Listening,
            DiscordActivity::Watching => ActivityType::Watching,
            DiscordActivity::Competing => ActivityType::Competing,
        };
        let activity = Activity {
            details: Some(details.into()),
            state: Some(state.into()),
            activity_type: Some(activity_type),
            buttons: vec![ActivityButton {
                label: Some("Доступ в интернет".into()),
                url: Some(PROJECT_URL.into()),
            }],
            instance: Some(true),
            ..Default::default()
        };
        *self
            .desired_activity
            .lock()
            .map_err(|_| anyhow::anyhow!("discord activity lock poisoned"))? =
            Some(activity.clone());

        if self.client.is_none() {
            let mut client = Client::new(CLIENT_ID);
            let desired_activity = Arc::clone(&self.desired_activity);
            let ready_client = client.clone();
            client
                .on_ready(move |_| {
                    let activity = match desired_activity.lock() {
                        Ok(activity) => activity.clone(),
                        Err(_) => {
                            eprintln!("discord activity lock poisoned");
                            return;
                        }
                    };
                    if let Some(activity) = activity {
                        let mut client = ready_client.clone();
                        if let Err(error) = client.set_activity(|_| activity) {
                            eprintln!("не удалось обновить Discord Presence: {error}");
                        }
                    }
                })
                .persist();
            client.start();
            self.client = Some(client);
        }
        if !Client::is_ready() {
            return Ok(());
        }
        let client = self
            .client
            .as_mut()
            .context("Discord Presence client не инициализирован")?;
        client
            .set_activity(|_| activity)
            .context("не удалось обновить Discord Presence")?;
        self.last_key = Some(key);
        Ok(())
    }
}
