use anyhow::Context as _;
use gpui::*;

use super::AppState;
use crate::services::binaries::{
    check_local_health, check_remote_updates, refresh_stale_lists, repair_managed_files,
};
use crate::services::file_watcher::{FileWatchEvent, start as start_file_watcher};

impl AppState {
    pub fn start_files_watcher(
        &self,
    ) -> anyhow::Result<tokio::sync::mpsc::UnboundedReceiver<FileWatchEvent>> {
        start_file_watcher(self.repository.resources_dir())
    }

    pub fn maintain_managed_files(&mut self, refresh_remote_lists: bool, cx: &mut Context<Self>) {
        if self.checking_files || self.download_progress.is_some() {
            return;
        }
        self.checking_files = true;
        self.log(if refresh_remote_lists {
            "Проверяю управляемые файлы и обновления списков..."
        } else {
            "Проверяю изменённые управляемые файлы..."
        });
        cx.notify();

        let client = self.http_client.clone();
        let resources_dir = self.repository.resources_dir().to_path_buf();
        let repository = self.repository.clone();
        let config = self.config.clone();
        cx.spawn(async move |entity, cx| {
            let result = crate::services::run_tokio(async move {
                let repaired = repair_managed_files(&client, &resources_dir).await;
                let repaired_filters =
                    tokio::task::spawn_blocking(move || repository.repair_filter_files(&config))
                        .await
                        .context("задача восстановления фильтров завершилась аварийно")
                        .and_then(|result| result);
                let refreshed_lists = if refresh_remote_lists {
                    Some(refresh_stale_lists(&client, &resources_dir).await)
                } else {
                    None
                };
                let health = if refresh_remote_lists {
                    check_remote_updates(&client, &resources_dir).await
                } else {
                    Ok(check_local_health(&resources_dir))
                };
                Ok((repaired, repaired_filters, refreshed_lists, health))
            })
            .await;
            let _update_result = entity.update(cx, |state, cx| {
                state.checking_files = false;
                match result {
                    Ok((repaired, repaired_filters, refreshed_lists, health)) => {
                        match repaired {
                            Ok(files) if !files.is_empty() => state.log(&format!(
                                "Автоматически восстановлены файлы: {}",
                                files.join(", ")
                            )),
                            Ok(_) => {}
                            Err(error) => state.log(&format!(
                                "Не удалось автоматически восстановить файлы: {error:#}"
                            )),
                        }
                        match repaired_filters {
                            Ok(files) if !files.is_empty() => state.log(&format!(
                                "Автоматически восстановлены фильтры: {}",
                                files.join(", ")
                            )),
                            Ok(_) => {}
                            Err(error) => state.log(&format!(
                                "Не удалось автоматически восстановить фильтры: {error:#}"
                            )),
                        }
                        if let Some(refreshed_lists) = refreshed_lists {
                            match refreshed_lists {
                                Ok(files) if !files.is_empty() => state.log(&format!(
                                    "Списки обновлены автоматически: {}",
                                    files.join(", ")
                                )),
                                Ok(_) => {}
                                Err(error) => state.log(&format!(
                                    "Не удалось автоматически обновить списки: {error:#}"
                                )),
                            }
                        }
                        match health {
                            Ok(mut snapshot) => {
                                if !refresh_remote_lists {
                                    snapshot.available_updates =
                                        state.health.available_updates.clone();
                                }
                                let has_issues = !snapshot.missing_critical_files.is_empty()
                                    || !snapshot.available_updates.is_empty();
                                state.health = snapshot;
                                state.log(if has_issues {
                                    "Обнаружены отсутствующие или требующие обновления файлы"
                                } else {
                                    "Все управляемые файлы и списки актуальны"
                                });
                            }
                            Err(error) => state.log(&format!(
                                "Не удалось проверить удалённые обновления файлов: {error:#}"
                            )),
                        }
                    }
                    Err(error) => state.log(&format!(
                        "Фоновая проверка управляемых файлов прервана: {error:#}"
                    )),
                }
                cx.notify();
            });
        })
        .detach();
    }
}
