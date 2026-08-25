use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use anyhow::{Context as _, Result};
use gpui::*;
use reqwest::Client;

use crate::domain::*;
use crate::services::binaries::{
    AppHealthSnapshot, DownloadProgress, check_local_health, repair_default_filters_for_bootstrap,
};
use crate::services::updater::AppUpdateInfo;
use crate::services::{RuntimeServices, cleanup_orphaned_processes, is_elevated};
use crate::ui::components::confirm_dialog::ConfirmTarget;
use crate::ui::foundation::colors::{self, ThemeMode};

mod collections;
mod filters;
mod managed_files;
mod probe;
mod runtime;

pub use probe::StrategyProbeState;

const MAX_LOGS: usize = 500;

pub struct AppState {
    pub config: AppConfig,
    pub builtin: AppConfig,
    pub status: ConnectionStatus,
    pub logs: VecDeque<LogEntry>,
    pub error: Option<String>,
    pub autostart_enabled: bool,
    pub dns_latencies: HashMap<String, Option<u128>>,
    pub dns_checking: bool,
    pub health: AppHealthSnapshot,
    pub download_progress: Option<DownloadProgress>,
    pub app_update: Option<AppUpdateInfo>,
    pub app_update_checked: bool,
    pub app_update_error: Option<String>,
    pub is_updating: bool,
    pub update_download_progress: Option<f32>,
    pub checking_app_update: bool,
    pub checking_files: bool,
    pub strategy_probe: StrategyProbeState,
    pending_restart: bool,
    probe_reconnect_pending: bool,
    probe_cancel: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    quit_after_probe: bool,
    pub confirm: Option<ConfirmTarget>,
    pub http_client: Client,
    repository: ConfigRepository,
    runtime: Arc<RuntimeServices>,
}

impl AppState {
    pub fn load() -> Result<Self> {
        let mut initial_logs = vec!["Запускаю инициализацию приложения".to_owned()];

        let repository = ConfigRepository::portable()?;
        let resources_dir = repository.resources_dir().to_path_buf();
        if !resources_dir.is_dir() {
            anyhow::bail!("папка thirdparty не найдена: {}", resources_dir.display());
        }
        initial_logs.push("Папка thirdparty найдена".to_owned());
        let http_client = Client::builder()
            .build()
            .context("не удалось создать HTTP-клиент")?;
        if !repository.config_path().is_file() {
            let restored = crate::services::async_runtime::TOKIO
                .block_on(repair_default_filters_for_bootstrap(
                    &http_client,
                    &resources_dir,
                ))
                .context("не удалось восстановить системные фильтры перед созданием config.json")?;
            if !restored.is_empty() {
                initial_logs.push(format!(
                    "До загрузки конфигурации восстановлены файлы: {}",
                    restored.join(", ")
                ));
            }
        }
        match cleanup_orphaned_processes(&repository) {
            Ok(()) => initial_logs.push("Управляемые процессы прошлого запуска очищены".to_owned()),
            Err(error) => initial_logs.push(format!(
                "Не удалось полностью очистить прошлый запуск: {error:#}"
            )),
        }
        initial_logs.push("Загружаю конфигурацию".to_owned());
        let config = repository.load_or_create()?;
        let mut recovery =
            match crate::services::probe::load_recovery_journal(repository.runtime_dir()) {
                Ok(recovery) => recovery,
                Err(error) => {
                    initial_logs.push(format!("Не удалось прочитать журнал подбора: {error:#}"));
                    let _clear_result =
                        crate::services::probe::clear_recovery_journal(repository.runtime_dir());
                    None
                }
            };
        if recovery
            .as_ref()
            .is_some_and(|journal| !journal.was_connected)
        {
            crate::services::probe::clear_recovery_journal(repository.runtime_dir())?;
            recovery = None;
        }
        let restored_filters = repository.repair_filter_files(&config)?;
        if !restored_filters.is_empty() {
            initial_logs.push(format!(
                "Из конфигурации восстановлены фильтры: {}",
                restored_filters.join(", ")
            ));
        }
        initial_logs.push("Конфигурация загружена".to_owned());
        apply_theme(config.theme);
        apply_language(config.language);
        let builtin = repository.builtin()?;
        initial_logs.push("Системная конфигурация загружена".to_owned());
        let runtime = Arc::new(RuntimeServices::new(repository.clone())?);
        let autostart_enabled = runtime.is_autostart_enabled();
        initial_logs.push("Настройки автозапуска Windows проверены".to_owned());
        let elevated = is_elevated();
        if elevated {
            initial_logs.push("Приложение запущено с правами администратора".to_owned());
        } else {
            initial_logs.push("Приложение запущено без прав администратора".to_owned());
        }

        let system_status = runtime.initialize_system();
        let health = check_local_health(&resources_dir);

        let mut state = Self {
            builtin,
            runtime,
            config,
            status: ConnectionStatus::Disconnected,
            logs: VecDeque::new(),
            error: None,
            autostart_enabled,
            dns_latencies: HashMap::new(),
            dns_checking: false,
            health,
            download_progress: None,
            app_update: None,
            app_update_checked: false,
            app_update_error: None,
            is_updating: false,
            update_download_progress: None,
            checking_app_update: false,
            checking_files: false,
            strategy_probe: StrategyProbeState::Idle,
            pending_restart: false,
            probe_reconnect_pending: recovery
                .as_ref()
                .is_some_and(|journal| journal.was_connected),
            probe_cancel: None,
            quit_after_probe: false,
            confirm: None,
            http_client,
            repository,
        };

        for message in initial_logs {
            state.log(&message);
        }
        match system_status {
            Ok(message) => state.log(message),
            Err(error) => {
                state.error = Some(format!("{error:#}"));
                state.log(&format!("Ошибка инициализации Windows: {error:#}"));
            }
        }
        state.log("Инициализация приложения завершена");
        if recovery.is_some() {
            state.log("Обнаружен незавершённый подбор стратегий; подключение будет восстановлено");
        }
        Ok(state)
    }

    pub fn mutate(&mut self, change: impl FnOnce(&mut AppConfig), cx: &mut Context<Self>) {
        change(&mut self.config);
        self.persist(cx);
    }

    pub fn set_theme(&mut self, theme: ThemePreference, cx: &mut Context<Self>) {
        if self.config.theme == theme {
            return;
        }
        self.config.theme = theme;
        apply_theme(theme);
        self.persist(cx);
    }

    pub fn set_language(&mut self, language: LanguagePreference, cx: &mut Context<Self>) {
        if self.config.language == language {
            return;
        }
        self.config.language = language;
        apply_language(language);
        self.persist(cx);
    }

    pub fn set_acrylic_material(&mut self, enabled: bool, cx: &mut Context<Self>) {
        if self.config.acrylic_material == enabled {
            return;
        }
        self.config.acrylic_material = enabled;
        self.persist(cx);
    }

    pub fn set_sidebar_collapsed(&mut self, collapsed: bool, cx: &mut Context<Self>) {
        if self.config.sidebar_collapsed == collapsed {
            return;
        }
        self.config.sidebar_collapsed = collapsed;
        self.persist(cx);
    }

    pub fn set_confirm(&mut self, target: Option<ConfirmTarget>, cx: &mut Context<Self>) {
        self.confirm = target;
        cx.notify();
    }

    fn persist(&mut self, cx: &mut Context<Self>) {
        match self
            .repository
            .save(&self.config)
            .context("не удалось сохранить настройки")
        {
            Ok(()) => self.error = None,
            Err(error) => self.error = Some(format!("{error:#}")),
        }
        cx.notify();
    }

    pub fn set_error(&mut self, error: anyhow::Error, cx: &mut Context<Self>) {
        let msg = format!("{error:#}");
        self.error = Some(msg.clone());
        self.log(&format!("Ошибка: {msg}"));
        cx.notify();
    }

    pub fn is_portable(&self) -> bool {
        self.repository.is_portable()
    }

    pub fn log(&mut self, message: &str) {
        self.logs.push_back(LogEntry {
            timestamp: std::time::SystemTime::now(),
            message: message.to_owned(),
        });
        while self.logs.len() > MAX_LOGS {
            self.logs.pop_front();
        }
    }
}

fn apply_theme(theme: ThemePreference) {
    colors::set_active_theme(match theme {
        ThemePreference::System => ThemeMode::System,
        ThemePreference::Dark => ThemeMode::Dark,
        ThemePreference::Light => ThemeMode::Light,
    });
}

fn apply_language(language: LanguagePreference) {
    let locale = match language {
        LanguagePreference::System => crate::ui::foundation::i18n::detect_system_language(),
        LanguagePreference::Ru => "ru",
        LanguagePreference::En => "en",
    };
    crate::ui::foundation::i18n::set_language(locale);
}
