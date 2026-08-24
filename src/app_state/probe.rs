use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use gpui::*;

use crate::domain::ConnectionStatus;
use crate::services::probe::{
    ProbeMode, ProbeProgress, ProbeReport, ProbeRequest, run_strategy_probe,
};

use super::AppState;

#[derive(Clone, Debug)]
pub enum StrategyProbeState {
    Idle,
    Running(ProbeProgress),
    Complete(ProbeReport),
    Error(String),
}

enum ProbeEvent {
    Progress(ProbeProgress),
    Finished(anyhow::Result<ProbeReport>),
}

impl AppState {
    pub fn start_strategy_probe(
        &mut self,
        category_ids: Vec<String>,
        mode: ProbeMode,
        cx: &mut Context<Self>,
    ) {
        if self.probe_cancel.is_some() || category_ids.is_empty() {
            return;
        }
        let was_connected = matches!(self.status, ConnectionStatus::Connected);
        let request = ProbeRequest {
            category_ids,
            mode,
            was_connected,
        };
        let cancelled = Arc::new(AtomicBool::new(false));
        self.probe_cancel = Some(cancelled.clone());
        self.strategy_probe = StrategyProbeState::Running(ProbeProgress::default());
        self.status = ConnectionStatus::Disconnecting;
        self.error = None;
        self.log("Запущен подбор стратегий");

        let resources_dir = self.repository.resources_dir().to_path_buf();
        let runtime_dir = self.repository.runtime_dir().to_path_buf();
        let runtime = self.runtime.clone();
        let original = self.config.clone();
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
        cx.background_executor()
            .spawn(async move {
                let progress_sender = sender.clone();
                let result = run_strategy_probe(
                    &resources_dir,
                    &runtime_dir,
                    &runtime,
                    &original,
                    &request,
                    cancelled,
                    move |progress| {
                        let _send_result = progress_sender.send(ProbeEvent::Progress(progress));
                    },
                );
                let _send_result = sender.send(ProbeEvent::Finished(result));
            })
            .detach();

        cx.spawn(async move |entity, cx| {
            while let Some(event) = receiver.recv().await {
                let finished = matches!(event, ProbeEvent::Finished(_));
                let _update_result = entity.update(cx, |state, cx| match event {
                    ProbeEvent::Progress(progress) => {
                        state.strategy_probe = StrategyProbeState::Running(progress);
                        cx.notify();
                    }
                    ProbeEvent::Finished(result) => {
                        state.probe_cancel = None;
                        state.status = if was_connected {
                            ConnectionStatus::Connected
                        } else {
                            ConnectionStatus::Disconnected
                        };
                        match result {
                            Ok(report) => {
                                state.log("Подбор стратегий завершён");
                                for recommendation in &report.recommendations {
                                    state.log(&format!(
                                        "{}: рекомендована {} (проверено {})",
                                        recommendation.category_name,
                                        recommendation.strategy_name,
                                        recommendation.candidates_tested
                                    ));
                                }
                                state.strategy_probe = StrategyProbeState::Complete(report);
                            }
                            Err(error) => {
                                let message = format!("Подбор стратегий: {error:#}");
                                state.log(&message);
                                state.strategy_probe = StrategyProbeState::Error(message);
                            }
                        }
                        if was_connected && state.pending_restart && !state.quit_after_probe {
                            state.pending_restart = false;
                            state.apply_connected(cx);
                        }
                        if state.quit_after_probe {
                            cx.quit();
                        } else {
                            cx.notify();
                        }
                    }
                });
                if finished {
                    break;
                }
            }
        })
        .detach();
        cx.notify();
    }

    pub fn cancel_strategy_probe(&mut self, cx: &mut Context<Self>) {
        if let Some(cancelled) = &self.probe_cancel {
            cancelled.store(true, Ordering::Relaxed);
            self.log("Отмена подбора стратегий…");
            cx.notify();
        }
    }

    pub fn defer_quit_for_probe(&mut self, cx: &mut Context<Self>) -> bool {
        if self.probe_cancel.is_none() {
            return false;
        }
        self.quit_after_probe = true;
        self.cancel_strategy_probe(cx);
        true
    }

    pub fn apply_strategy_probe_report(&mut self, cx: &mut Context<Self>) {
        let StrategyProbeState::Complete(report) = &self.strategy_probe else {
            return;
        };
        let mut config = self.config.clone();
        for recommendation in &report.recommendations {
            let Some(category) = config
                .categories
                .iter_mut()
                .find(|category| category.id == recommendation.category_id)
            else {
                self.set_error(
                    anyhow::anyhow!(
                        "Категория {} больше не существует",
                        recommendation.category_name
                    ),
                    cx,
                );
                return;
            };
            for strategy in &mut category.strategies {
                strategy.active = recommendation
                    .strategy_id
                    .as_ref()
                    .is_some_and(|id| strategy.id == *id);
            }
        }
        for strategy in config
            .categories
            .iter()
            .flat_map(|category| &category.strategies)
        {
            if let Err(error) = self.repository.save_strategy(strategy) {
                self.set_error(
                    anyhow::anyhow!("Не удалось применить подбор: {error:#}"),
                    cx,
                );
                return;
            }
        }
        self.config = config;
        self.log("Рекомендованный набор стратегий применён");
        self.apply_connected(cx);
        cx.notify();
    }

    pub fn probe_reconnect_pending(&self) -> bool {
        self.probe_reconnect_pending
    }

    pub(super) fn strategy_edits_blocked(&mut self, cx: &mut Context<Self>) -> bool {
        if self.probe_cancel.is_none() {
            return false;
        }
        self.error = Some("Остановите подбор перед изменением стратегий".to_owned());
        cx.notify();
        true
    }

    pub(super) fn finish_probe_recovery(&mut self) {
        if !self.probe_reconnect_pending {
            return;
        }
        match crate::services::probe::clear_recovery_journal(self.repository.runtime_dir()) {
            Ok(()) => {
                self.probe_reconnect_pending = false;
                self.log("Подключение после незавершённого подбора восстановлено");
            }
            Err(error) => self.log(&format!("Не удалось удалить журнал подбора: {error:#}")),
        }
    }
}
