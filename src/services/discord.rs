use anyhow::{Context as _, Result};
use discord_presence::Client;
use discord_presence::models::rich_presence::{Activity, ActivityButton, ActivityType};

use crate::domain::{AppConfig, DiscordActivity};

const CLIENT_ID: u64 = 1_495_773_045_904_769_255;
const PROJECT_URL: &str = "https://github.com/Noktomezo/ZapretInteractive";

#[derive(Default)]
pub struct DiscordPresence {
    client: Option<Client>,
    last_key: Option<String>,
}

impl DiscordPresence {
    pub fn sync(&mut self, config: &AppConfig, connected: bool) -> Result<()> {
        if !config.discord_presence_enabled {
            if let Some(mut client) = self.client.take() {
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
        let client = self.client.get_or_insert_with(|| {
            let mut client = Client::new(CLIENT_ID);
            client.start();
            client
        });
        let activity_type = match config.discord_presence_activity_type {
            DiscordActivity::Playing => ActivityType::Playing,
            DiscordActivity::Listening => ActivityType::Listening,
            DiscordActivity::Watching => ActivityType::Watching,
            DiscordActivity::Competing => ActivityType::Competing,
        };
        client
            .set_activity(|_| Activity {
                details: Some(details.into()),
                state: Some(state.into()),
                activity_type: Some(activity_type),
                buttons: vec![ActivityButton {
                    label: Some("Доступ в интернет".into()),
                    url: Some(PROJECT_URL.into()),
                }],
                instance: Some(true),
                ..Default::default()
            })
            .context("не удалось обновить Discord Presence")?;
        self.last_key = Some(key);
        Ok(())
    }
}
