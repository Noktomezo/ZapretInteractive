use anyhow::Context as _;
use gpui::*;

use super::AppState;
use crate::domain::{ConnectionStatus, DiscordActivity, ListMode, validate_port_spec};
use crate::services::binaries::{check_local_health, download_missing_or_outdated_files};
use crate::services::updater::check_app_update;
use crate::ui::components::confirm_dialog::ConfirmTarget;

#[derive(Clone, Copy)]
enum ManagedModule {
    Dns,
    TgProxy,
}

impl AppState {
    pub fn toggle_connection(&mut self, cx: &mut Context<Self>) {
        if matches!(
            self.status,
            ConnectionStatus::Connecting | ConnectionStatus::Disconnecting
        ) {
            return;
        }
        let connecting = !matches!(self.status, ConnectionStatus::Connected);
        self.status = if connecting {
            ConnectionStatus::Connecting
        } else {
            ConnectionStatus::Disconnecting
        };
        self.error = None;
        let runtime = self.runtime.clone();
        let config = self.config.clone();
        self.log(if connecting {
            "Запуск подключения…"
        } else {
            "Остановка подключения…"
        });
        cx.spawn(async move |entity, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    let action = if connecting {
                        runtime.connect(&config).map(|outcome| {
                            (
                                format!("winws запущен, PID {}", outcome.pid),
                                outcome.module_errors,
                            )
                        })
                    } else {
                        runtime
                            .disconnect()
                            .map(|()| ("Подключение остановлено".to_owned(), Vec::new()))
                    };
                    action.map(|(message, module_errors)| {
                        let discord_error = runtime
                            .sync_discord(&config, connecting)
                            .err()
                            .map(|error| format!("Discord Presence: {error:#}"));
                        (message, module_errors, discord_error)
                    })
                })
                .await;
            let _update_result = entity.update(cx, |state, cx| match result {
                Ok((message, module_errors, discord_error)) => {
                    state.status = if connecting {
                        ConnectionStatus::Connected
                    } else {
                        ConnectionStatus::Disconnected
                    };
                    state.log(&message);
                    for error in module_errors {
                        state.log(&format!("Ошибка модуля: {error}"));
                    }
                    if let Some(error) = discord_error {
                        state.error = Some(error.clone());
                        state.log(&error);
                    }
                    let restart = connecting && state.pending_restart;
                    state.pending_restart = false;
                    if restart {
                        state.apply_connected(cx);
                    }
                    cx.notify();
                }
                Err(error) => {
                    state.status = ConnectionStatus::Error;
                    state.error = Some(format!("{error:#}"));
                    state.log(&format!("Ошибка: {error:#}"));
                    cx.notify();
                }
            });
        })
        .detach();
        cx.notify();
    }

    pub fn apply_connected(&mut self, cx: &mut Context<Self>) {
        if matches!(
            self.status,
            ConnectionStatus::Connecting | ConnectionStatus::Disconnecting
        ) {
            self.pending_restart = true;
            return;
        }
        if !matches!(self.status, ConnectionStatus::Connected) {
            return;
        }
        self.status = ConnectionStatus::Connecting;
        let runtime = self.runtime.clone();
        let config = self.config.clone();
        self.log("Применение изменений…");
        cx.spawn(async move |entity, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    runtime.disconnect()?;
                    runtime.connect(&config)
                })
                .await;
            let _update_result = entity.update(cx, |state, cx| {
                match result {
                    Ok(outcome) => {
                        state.status = ConnectionStatus::Connected;
                        state.log(&format!("Изменения применены, PID {}", outcome.pid));
                        for error in outcome.module_errors {
                            state.log(&format!("Ошибка модуля: {error}"));
                        }
                        if state.pending_restart {
                            state.pending_restart = false;
                            state.apply_connected(cx);
                        }
                    }
                    Err(error) => {
                        state.status = ConnectionStatus::Error;
                        state.error = Some(format!("{error:#}"));
                        state.log(&format!("Ошибка применения: {error:#}"));
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub fn set_list_mode(&mut self, mode: ListMode, cx: &mut Context<Self>) {
        if self.config.list_mode == mode {
            return;
        }
        self.config.list_mode = mode;
        let mode_label = match mode {
            ListMode::Ipset => "Только заблокированные",
            ListMode::Exclude => "Исключения",
        };
        self.log(&format!("Режим списков изменен на: {mode_label}"));
        self.persist(cx);
        self.apply_connected(cx);
    }

    pub fn set_global_ports(&mut self, tcp: &str, udp: &str, cx: &mut Context<Self>) {
        if let Err(error) = validate_port_spec(tcp).and_then(|()| validate_port_spec(udp)) {
            self.set_error(error, cx);
            return;
        }
        if self.config.global_ports.tcp == tcp && self.config.global_ports.udp == udp {
            return;
        }
        self.config.global_ports.tcp = tcp.to_owned();
        self.config.global_ports.udp = udp.to_owned();
        self.log("Обновлены глобальные порты");
        self.persist(cx);
        self.apply_connected(cx);
    }

    pub fn set_dns_module(&mut self, enabled: bool, cx: &mut Context<Self>) {
        if self.config.dns_module_enabled == enabled {
            return;
        }
        self.config.dns_module_enabled = enabled;
        self.log(if enabled {
            "DoH DNS модуль включен"
        } else {
            "DoH DNS модуль выключен"
        });
        self.persist(cx);
        self.apply_module(ManagedModule::Dns, cx);
    }

    pub fn set_dns_preset(&mut self, preset_id: &str, cx: &mut Context<Self>) {
        if self.config.dns_accelerator_enabled || self.config.dns_preset_id == preset_id {
            return;
        }
        self.config.dns_preset_id = preset_id.to_owned();
        self.persist(cx);
        self.apply_module(ManagedModule::Dns, cx);
    }

    pub fn set_dns_bootstrap_resolvers(&mut self, resolvers: &[String], cx: &mut Context<Self>) {
        if self.config.dns_bootstrap_resolvers == resolvers {
            return;
        }
        self.config.dns_bootstrap_resolvers = resolvers.to_vec();
        self.persist(cx);
        self.apply_module(ManagedModule::Dns, cx);
    }

    pub fn set_tg_proxy_module(&mut self, enabled: bool, cx: &mut Context<Self>) {
        if self.config.tg_ws_proxy_module_enabled == enabled {
            return;
        }
        self.config.tg_ws_proxy_module_enabled = enabled;
        self.log(if enabled {
            "Telegram WS Proxy модуль включен"
        } else {
            "Telegram WS Proxy модуль выключен"
        });
        self.persist(cx);
        self.apply_module(ManagedModule::TgProxy, cx);
    }

    pub fn set_dns_multiqueue(&mut self, enabled: bool, cx: &mut Context<Self>) {
        if self.config.dns_accelerator_enabled == enabled {
            return;
        }
        self.config.dns_accelerator_enabled = enabled;
        self.persist(cx);
        self.apply_module(ManagedModule::Dns, cx);
    }

    fn apply_module(&mut self, module: ManagedModule, cx: &mut Context<Self>) {
        if matches!(
            self.status,
            ConnectionStatus::Connecting | ConnectionStatus::Disconnecting
        ) {
            self.pending_restart = true;
            return;
        }
        if !matches!(self.status, ConnectionStatus::Connected) {
            return;
        }
        let runtime = self.runtime.clone();
        let config = self.config.clone();
        cx.spawn(async move |entity, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    match module {
                        ManagedModule::Dns => runtime.sync_dns(&config),
                        ManagedModule::TgProxy => runtime.sync_tg_proxy(&config),
                    }
                })
                .await;
            let _update_result = entity.update(cx, |state, cx| {
                match result {
                    Ok(()) => state.log(match module {
                        ManagedModule::Dns => "Настройки DNS-модуля применены",
                        ManagedModule::TgProxy => "Настройки Telegram WS Proxy применены",
                    }),
                    Err(error) => state.set_error(error, cx),
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub fn set_minimize_to_tray(&mut self, enabled: bool, cx: &mut Context<Self>) {
        if self.config.minimize_to_tray == enabled {
            return;
        }
        self.config.minimize_to_tray = enabled;
        self.persist(cx);
    }

    pub fn set_launch_to_tray(&mut self, enabled: bool, cx: &mut Context<Self>) {
        if self.config.launch_to_tray == enabled {
            return;
        }
        self.config.launch_to_tray = enabled;
        self.persist(cx);
    }

    pub fn set_connect_on_autostart(&mut self, enabled: bool, cx: &mut Context<Self>) {
        if self.config.connect_on_autostart == enabled {
            return;
        }
        self.config.connect_on_autostart = enabled;
        self.persist(cx);
    }

    pub fn set_core_file_update_prompts(&mut self, enabled: bool, cx: &mut Context<Self>) {
        if self.config.core_file_update_prompts_enabled == enabled {
            return;
        }
        self.config.core_file_update_prompts_enabled = enabled;
        self.persist(cx);
    }

    pub fn set_app_auto_updates(&mut self, enabled: bool, cx: &mut Context<Self>) {
        if self.config.app_auto_updates_enabled == enabled {
            return;
        }
        self.config.app_auto_updates_enabled = enabled;
        self.persist(cx);
    }

    pub fn set_autostart(&mut self, enabled: bool, cx: &mut Context<Self>) {
        if self.autostart_enabled == enabled {
            return;
        }
        match self.runtime.set_autostart_enabled(enabled) {
            Ok(()) => {
                self.autostart_enabled = enabled;
                self.log(if enabled {
                    "Автозапуск Windows включен"
                } else {
                    "Автозапуск Windows выключен"
                });
                cx.notify();
            }
            Err(error) => self.set_error(error, cx),
        }
    }

    pub fn set_discord_presence(&mut self, enabled: bool, cx: &mut Context<Self>) {
        if self.config.discord_presence_enabled == enabled {
            return;
        }
        self.config.discord_presence_enabled = enabled;
        self.persist(cx);
        if let Err(error) = self.runtime.sync_discord(
            &self.config,
            matches!(self.status, ConnectionStatus::Connected),
        ) {
            self.set_error(error, cx);
        }
    }

    pub fn set_discord_activity(&mut self, activity: DiscordActivity, cx: &mut Context<Self>) {
        if self.config.discord_presence_activity_type == activity {
            return;
        }
        self.config.discord_presence_activity_type = activity;
        self.persist(cx);
        if let Err(error) = self.runtime.sync_discord(
            &self.config,
            matches!(self.status, ConnectionStatus::Connected),
        ) {
            self.set_error(error, cx);
        }
    }

    pub fn check_dns_latencies(&mut self, cx: &mut Context<Self>) {
        if self.dns_checking {
            return;
        }
        self.dns_checking = true;
        cx.spawn(async move |entity, cx| {
            let results = cx
                .background_executor()
                .spawn(async move { crate::services::dns::measure_preset_latencies() })
                .await;
            let _update_result = entity.update(cx, |state, cx| match results {
                Ok(results) => {
                    state.error = None;
                    state.dns_latencies = results;
                    state.dns_checking = false;
                    cx.notify();
                }
                Err(error) => {
                    state.error = Some(format!("{error:#}"));
                    state.dns_checking = false;
                    cx.notify();
                }
            });
        })
        .detach();
        cx.notify();
    }

    pub fn download_files(&mut self, cx: &mut Context<Self>) {
        if self.download_progress.is_some()
            || self.checking_files
            || matches!(
                self.status,
                ConnectionStatus::Connecting | ConnectionStatus::Disconnecting
            )
        {
            return;
        }
        let should_reconnect = matches!(self.status, ConnectionStatus::Connected);
        if should_reconnect {
            self.status = ConnectionStatus::Disconnecting;
        }
        let client = self.http_client.clone();
        let res_dir = self.repository.resources_dir().to_path_buf();
        let health_dir = res_dir.clone();
        let runtime = self.runtime.clone();
        let config = self.config.clone();
        self.error = None;
        self.download_progress = Some(crate::services::binaries::DownloadProgress {
            current: 0,
            total: 0,
            filename: String::new(),
        });
        self.log("Начинаю загрузку файлов thirdparty...");
        cx.spawn(async move |entity, cx| {
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
            let dl_task = crate::services::spawn_tokio(async move {
                if should_reconnect {
                    let pause_runtime = runtime.clone();
                    tokio::task::spawn_blocking(move || pause_runtime.disconnect())
                        .await
                        .context("задача остановки подключения завершилась аварийно")??;
                }
                let download_result =
                    download_missing_or_outdated_files(&client, &res_dir, move |progress| {
                        let _intentionally_ignored = tx.send(progress);
                    })
                    .await;
                let reconnect_result = if should_reconnect {
                    tokio::task::spawn_blocking(move || runtime.connect(&config))
                        .await
                        .context("задача восстановления подключения завершилась аварийно")?
                        .map(Some)
                } else {
                    Ok(None)
                };
                match (download_result, reconnect_result) {
                    (Ok(()), Ok(outcome)) => Ok((outcome, None)),
                    (Err(download), Ok(outcome)) => Ok((outcome, Some(download))),
                    (Ok(()), Err(reconnect)) => Err(reconnect),
                    (Err(download), Err(reconnect)) => Err(anyhow::anyhow!(
                        "загрузка: {download:#}; переподключение: {reconnect:#}"
                    )),
                }
            });

            while let Some(progress) = rx.recv().await {
                let _update_result = entity.update(cx, |state, cx| {
                    state.download_progress = Some(progress);
                    cx.notify();
                });
            }

            let result = match dl_task.await {
                Ok(r) => r,
                Err(e) => Err(anyhow::anyhow!("Download task cancelled: {e}")),
            };
            let _update_result = entity.update(cx, |state, cx| {
                state.download_progress = None;
                match result {
                    Ok((outcome, download_error)) => {
                        if download_error.is_none() {
                            state.health = check_local_health(&health_dir);
                            state.log("Загрузка файлов успешно завершена");
                        }
                        if let Some(outcome) = outcome {
                            state.status = ConnectionStatus::Connected;
                            state.log(&format!("Подключение восстановлено, PID {}", outcome.pid));
                            for error in outcome.module_errors {
                                state.log(&format!("Ошибка модуля: {error}"));
                            }
                        }
                        if let Some(error) = download_error {
                            state.set_error(error, cx);
                        }
                    }
                    Err(e) => {
                        if should_reconnect {
                            state.status = ConnectionStatus::Error;
                        }
                        state.set_error(e, cx);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub fn check_app_updates(&mut self, cx: &mut Context<Self>) {
        if self.checking_app_update {
            return;
        }
        self.checking_app_update = true;
        self.app_update = None;
        self.app_update_checked = false;
        self.app_update_error = None;
        self.log("Проверяю доступность новой версии...");
        cx.notify();

        let client = self.http_client.clone();
        let current_version = env!("CARGO_PKG_VERSION");
        cx.spawn(async move |entity, cx| {
            let result = crate::services::run_tokio(async move {
                check_app_update(&client, current_version).await
            })
            .await;
            let _update_result = entity.update(cx, |state, cx| {
                state.checking_app_update = false;
                match result {
                    Ok(Some(update)) => {
                        state.log(&format!("Доступна новая версия: {}", update.new_version));
                        state.app_update = Some(update);
                        state.app_update_checked = true;
                    }
                    Ok(None) => {
                        state.log("У вас установлена последняя версия Zapret Interactive");
                        state.app_update_checked = true;
                    }
                    Err(e) => {
                        state.log(&format!("Не удалось проверить обновления: {e:#}"));
                        state.app_update_error = Some(format!("{e:#}"));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub fn trigger_app_update(&mut self, cx: &mut Context<Self>) {
        if self.is_updating {
            return;
        }
        let Some(update) = self.app_update.clone() else {
            return;
        };

        let Some(download_url) = update.download_url else {
            self.open_external(&update.release_url, cx);
            return;
        };

        self.is_updating = true;
        self.app_update_error = None;
        self.update_download_progress = Some(0.0);
        self.log(&format!(
            "Начинаю загрузку обновления {}...",
            update.new_version
        ));
        cx.notify();

        let client = self.http_client.clone();
        cx.spawn(async move |entity, cx| {
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
            let update_task = crate::services::spawn_tokio(async move {
                crate::services::updater::download_and_install_app_update(
                    &client,
                    &download_url,
                    move |progress| {
                        let _send_result = tx.send(progress);
                    },
                )
                .await
            });

            while let Some(progress) = rx.recv().await {
                let _update_result = entity.update(cx, |state, cx| {
                    state.update_download_progress = Some(progress);
                    cx.notify();
                });
            }

            let result = match update_task.await {
                Ok(r) => r,
                Err(e) => Err(anyhow::anyhow!("Update task cancelled: {e}")),
            };

            let _update_result = entity.update(cx, |state, cx| {
                state.is_updating = false;
                state.update_download_progress = None;
                if let Err(e) = result {
                    let error = format!("{e:#}");
                    state.log(&format!("Не удалось установить обновление: {error}"));
                    state.app_update_error = Some(error);
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub fn execute_confirm(&mut self, cx: &mut Context<Self>) {
        if let Some(target) = self.confirm.take() {
            match target {
                ConfirmTarget::DeleteCategory { id, .. } => {
                    self.delete_category(&id, cx);
                }
                ConfirmTarget::DeleteStrategy {
                    category_id,
                    strategy_id,
                    ..
                } => {
                    self.delete_strategy(&category_id, &strategy_id, cx);
                }
                ConfirmTarget::DeleteFilter { id, .. } => {
                    self.delete_filter(&id, cx);
                }
                ConfirmTarget::DeletePlaceholder { index, .. } => {
                    self.delete_placeholder(index, cx);
                }
                ConfirmTarget::ResetConfig => {
                    self.reset_to_defaults(cx);
                }
            }
        }
    }

    pub fn open_filters_directory(&mut self, cx: &mut Context<Self>) {
        if let Err(e) = self.runtime.open_filters_directory() {
            self.set_error(e, cx);
        }
    }

    pub fn open_placeholders_directory(&mut self, cx: &mut Context<Self>) {
        if let Err(e) = self.runtime.open_placeholders_directory() {
            self.set_error(e, cx);
        }
    }

    pub fn set_ports(&mut self, tcp: String, udp: String, cx: &mut Context<Self>) {
        self.set_global_ports(&tcp, &udp, cx);
    }

    pub fn set_tg_settings(&mut self, port_str: String, secret: String, cx: &mut Context<Self>) {
        let port = match port_str.trim().parse::<u16>() {
            Ok(port) if port != 0 => port,
            _ => {
                self.set_error(
                    anyhow::anyhow!("порт TG-прокси должен быть от 1 до 65535"),
                    cx,
                );
                return;
            }
        };
        let normalized_secret = secret.trim().to_ascii_lowercase();
        if normalized_secret.len() != 32
            || !normalized_secret
                .chars()
                .all(|character| character.is_ascii_hexdigit())
        {
            self.set_error(
                anyhow::anyhow!("секрет TG-прокси должен содержать 32 hex-символа"),
                cx,
            );
            return;
        }
        if self.config.tg_ws_proxy_port == port
            && self.config.tg_ws_proxy_secret == normalized_secret
        {
            return;
        }
        self.config.tg_ws_proxy_port = port;
        self.config.tg_ws_proxy_secret = normalized_secret;
        self.persist(cx);
        self.apply_module(ManagedModule::TgProxy, cx);
    }

    pub fn reset_to_defaults(&mut self, cx: &mut Context<Self>) {
        match self.repository.reset() {
            Ok(builtin) => {
                self.config = builtin;
                self.apply_connected(cx);
                self.log("Настройки сброшены к значениям по умолчанию");
            }
            Err(e) => self.set_error(e, cx),
        }
    }

    pub fn open_external(&mut self, url: &str, cx: &mut Context<Self>) {
        if let Err(e) = self.runtime.open_external(url) {
            self.set_error(e, cx);
        }
    }

    pub fn tg_proxy_pid(&self) -> Option<u32> {
        self.runtime.tg_proxy_pid()
    }

    pub fn clear_logs(&mut self, cx: &mut Context<Self>) {
        self.logs.clear();
        cx.notify();
    }
}
