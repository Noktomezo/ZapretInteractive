use super::*;
use crate::domain::ConnectionStatus;
use std::time::Duration;

impl AppView {
    pub fn render_page(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let config = self.state.read(cx).config.clone();
        match self.route.clone() {
            Route::Home => self.home(cx),
            Route::Modules => page(
                t!("modules.title"),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(interactive_module_card(
                        t!("modules.dns_title"),
                        t!("modules.dns_desc"),
                        ("icons/globe.svg", accent()),
                        config.dns_module_enabled,
                        cx.listener(|this, _, _, cx| this.navigate(Route::Dns, cx)),
                        {
                            let state = self.state.clone();
                            move |_, _, cx| {
                                state.update(cx, |state, cx| {
                                    state.set_dns_module(!config.dns_module_enabled, cx);
                                });
                            }
                        },
                        cx,
                    ))
                    .child(interactive_module_card(
                        t!("modules.tg_proxy_title"),
                        t!("modules.tg_proxy_desc"),
                        ("icons/send.svg", rgba(0x4385beff)),
                        config.tg_ws_proxy_module_enabled,
                        cx.listener(|this, _, _, cx| this.navigate(Route::TgProxy, cx)),
                        {
                            let state = self.state.clone();
                            move |_, _, cx| {
                                state.update(cx, |state, cx| {
                                    state.set_tg_proxy_module(
                                        !config.tg_ws_proxy_module_enabled,
                                        cx,
                                    );
                                });
                            }
                        },
                        cx,
                    )),
            ),
            Route::Dns => self.dns_page(cx),
            Route::TgProxy => self.tg_page(cx),
            Route::Strategies => self.strategies_page(cx),
            Route::Category(id) => self.category_page(&id, cx),
            Route::Filters => self.filters_page(cx),
            Route::Placeholders => self.placeholders_page(cx),
            Route::Logs => self.logs_page(cx),
            Route::Settings => self.settings_page(cx),
            Route::About => self.about_page(cx),
        }
        .into_any_element()
    }

    fn home(&self, cx: &mut Context<Self>) -> AnyElement {
        if let Some(block) = critical_files_block(self.state.clone(), cx) {
            return block;
        }

        let state = self.state.read(cx);
        let status = state.status;
        let mode = state.config.list_mode;

        let status_color = match status {
            ConnectionStatus::Connected => success(),
            ConnectionStatus::Connecting | ConnectionStatus::Disconnecting => warning(),
            ConnectionStatus::Error => danger(),
            ConnectionStatus::Disconnected => foreground(),
        };
        let app_state = self.state.clone();
        let app_state_mode = self.state.clone();
        let glass_border = mix_color(rgba(0xffffff33), status_color.opacity(0.18), 0.5);
        let glass_shine = mix_color(rgba(0xffffff2e), status_color.opacity(0.48), 0.62);
        let orbit_duration = if matches!(
            status,
            ConnectionStatus::Connecting | ConnectionStatus::Disconnecting
        ) {
            Duration::from_millis(1_200)
        } else {
            Duration::from_millis(2_400)
        };

        div()
            .size_full()
            .relative()
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .p_6()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap_5()
                            .child(
                                div()
                                    .id("connect-button-orbit")
                                    .group("connect-glass")
                                    .relative()
                                    .size(px(128.))
                                    .child(
                                        div()
                                            .id("connect-button")
                                            .absolute()
                                            .inset_0()
                                            .rounded_full()
                                            .overflow_hidden()
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .cursor_pointer()
                                            .border_1()
                                            .border_color(glass_border)
                                            .shadow_lg()
                                            .child(backdrop_blur(
                                                card_color().opacity(0.26).into(),
                                                px(20.0),
                                                px(64.0),
                                                0.012,
                                            ))
                                            .child(
                                                div()
                                                    .absolute()
                                                    .inset(px(1.))
                                                    .rounded_full()
                                                    .border_t_1()
                                                    .border_color(rgba(0xffffff4d)),
                                            )
                                            .on_click(move |_, _, cx| {
                                                app_state.update(cx, |state, cx| {
                                                    state.toggle_connection(cx)
                                                })
                                            })
                                            .child(
                                                svg()
                                                    .path("icons/power.svg")
                                                    .size(px(48.))
                                                    .text_color(status_color)
                                                    .opacity(0.76),
                                            ),
                                    )
                                    .child(
                                        svg()
                                            .absolute()
                                            .inset_0()
                                            .size(px(128.))
                                            .path("icons/glass-shine.svg")
                                            .text_color(glass_shine)
                                            .group_hover("connect-glass", |style| {
                                                style.opacity(0.0)
                                            })
                                            .with_animation(
                                                "connect-button-orbit",
                                                Animation::new(orbit_duration).repeat(),
                                                |shine, delta| {
                                                    let angle = delta * std::f32::consts::TAU;
                                                    shine.with_transformation(
                                                        Transformation::rotate(Radians(angle)),
                                                    )
                                                },
                                            ),
                                    ),
                            )
                            .child(
                                div()
                                    .text_2xl()
                                    .line_height(px(28.8))
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(status.label()),
                            )
                            .child(
                                div()
                                    .relative()
                                    .w(px(342.))
                                    .h(px(36.))
                                    .flex()
                                    .gap(px(2.))
                                    .p(px(2.))
                                    .rounded(px(8.))
                                    .overflow_hidden()
                                    .border_1()
                                    .border_color(border().opacity(0.6))
                                    .shadow_lg()
                                    .child(backdrop_blur(
                                        card_color().opacity(0.26).into(),
                                        px(20.0),
                                        px(8.0),
                                        0.012,
                                    ))
                                    .child(mode_indicator(mode, cx))
                                    .child(mode_item(
                                        "mode-ipset",
                                        t!("home.mode_ipset"),
                                        mode == ListMode::Ipset,
                                        success(),
                                        move |_, window, cx| {
                                            if matches!(status, ConnectionStatus::Disconnected) {
                                                animate_toggle(
                                                    "list-mode-indicator",
                                                    false,
                                                    window,
                                                    cx,
                                                );
                                                app_state_mode.update(cx, |state, cx| {
                                                    state.set_list_mode(ListMode::Ipset, cx)
                                                });
                                            }
                                        },
                                    ))
                                    .child(mode_item(
                                        "mode-exclude",
                                        t!("home.mode_exclude"),
                                        mode == ListMode::Exclude,
                                        warning(),
                                        {
                                            let state = self.state.clone();
                                            move |_, window, cx| {
                                                if matches!(status, ConnectionStatus::Disconnected)
                                                {
                                                    animate_toggle(
                                                        "list-mode-indicator",
                                                        true,
                                                        window,
                                                        cx,
                                                    );
                                                    state.update(cx, |state, cx| {
                                                        state.set_list_mode(ListMode::Exclude, cx)
                                                    });
                                                }
                                            }
                                        },
                                    )),
                            ),
                    )
                    .when_some(state.error.clone(), |root, error| {
                        root.child(
                            div()
                                .absolute()
                                .left_0()
                                .right_0()
                                .bottom(px(24.))
                                .flex()
                                .justify_center()
                                .child(
                                    div()
                                        .max_w(px(520.))
                                        .truncate()
                                        .text_xs()
                                        .text_color(danger())
                                        .child(error),
                                ),
                        )
                    }),
            )
            .into_any_element()
    }
}

fn critical_files_block(state: Entity<crate::app_state::AppState>, cx: &App) -> Option<AnyElement> {
    let (missing_files, checking, progress, error) = {
        let app_state = state.read(cx);
        if app_state.health.binaries_ok {
            return None;
        }
        (
            app_state.health.missing_critical_files.clone(),
            app_state.checking_files,
            app_state.download_progress.clone(),
            app_state.error.clone(),
        )
    };

    let downloading = progress.is_some();
    let busy = checking || downloading;
    let status_icon = if busy {
        svg()
            .path("icons/refresh-cw.svg")
            .size(px(28.))
            .text_color(colors::orange())
            .with_animation(
                "critical-files-loader",
                Animation::new(Duration::from_millis(850)).repeat(),
                |icon, delta| {
                    icon.with_transformation(Transformation::rotate(
                        crate::ui::foundation::motion::refresh_rotation(delta),
                    ))
                },
            )
            .into_any_element()
    } else {
        svg()
            .path("icons/triangle-alert.svg")
            .size(px(28.))
            .text_color(colors::orange())
            .with_animation(
                "critical-files-pulse",
                Animation::new(crate::ui::foundation::motion::UPDATE_PULSE_MOTION).repeat(),
                |icon, delta| {
                    icon.opacity(crate::ui::foundation::motion::update_pulse_opacity(delta))
                },
            )
            .into_any_element()
    };

    let status_badge = if let Some(progress) = progress.as_ref() {
        if let Some(fraction) = critical_download_fraction(progress.current, progress.total) {
            crate::ui::components::badge::progress_badge(
                t!(
                    "home.downloading_files",
                    current = progress.current,
                    total = progress.total
                ),
                fraction,
            )
        } else {
            crate::ui::components::badge::loading_badge(t!("home.downloading_critical_files"))
        }
    } else if checking {
        crate::ui::components::badge::loading_badge(t!("home.checking_critical_files"))
    } else {
        crate::ui::components::badge::Badge::new(t!("home.download_required"))
            .warning()
            .icon("icons/triangle-alert.svg")
            .into_any_element()
    };

    let files = missing_files.iter().take(4).cloned().collect::<Vec<_>>();
    let mut files_text = files.join(", ");
    if missing_files.len() > files.len() {
        files_text.push('…');
    }
    let description = t!("home.missing_binaries_desc", files = files_text.as_str());
    let action_state = state;
    let action_label = if downloading {
        t!("home.downloading_critical_files")
    } else if checking {
        t!("home.checking_critical_files")
    } else {
        t!("home.btn_download_binaries")
    };

    let content = div()
        .p_5()
        .flex()
        .flex_col()
        .items_center()
        .gap_4()
        .child(
            div()
                .size(px(64.))
                .flex()
                .items_center()
                .justify_center()
                .rounded_full()
                .border_1()
                .border_color(colors::orange().opacity(0.35))
                .bg(colors::orange().opacity(0.10))
                .child(status_icon),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .items_center()
                .gap_2()
                .text_center()
                .child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::MEDIUM)
                        .child(t!("home.missing_binaries_title")),
                )
                .child(
                    div()
                        .max_w(px(380.))
                        .text_xs()
                        .line_height(px(18.))
                        .text_color(muted_foreground())
                        .child(description),
                ),
        )
        .child(status_badge)
        .when_some(error, |content, error| {
            content.child(
                div()
                    .max_w(px(380.))
                    .text_xs()
                    .text_center()
                    .text_color(danger())
                    .child(error),
            )
        })
        .child(
            crate::ui::components::button::Button::new("download-critical-files", action_label, cx)
                .primary()
                .loading(busy)
                .icon_prefix("icons/cloud-download.svg")
                .on_click(move |_, _, cx| {
                    action_state.update(cx, |state, cx| state.download_files(cx));
                }),
        );

    Some(
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .p_8()
            .child(
                div().w_full().max_w(px(440.)).child(
                    crate::ui::components::card::Card::new()
                        .child(content)
                        .into_any_element(),
                ),
            )
            .into_any_element(),
    )
}

fn critical_download_fraction(current: usize, total: usize) -> Option<f32> {
    (total > 0).then(|| (current as f32 / total as f32).clamp(0.0, 1.0))
}

fn mode_indicator(mode: ListMode, cx: &App) -> AnyElement {
    let exclude = mode == ListMode::Exclude;
    let progress = crate::ui::foundation::hover_motion::state_progress(
        &"list-mode-indicator".into(),
        exclude,
        cx,
    );
    let color = if mode == ListMode::Ipset {
        success()
    } else {
        warning()
    };
    div()
        .absolute()
        .top(px(2.))
        .bottom(px(2.))
        .w(px(168.))
        .rounded(px(6.))
        .border_1()
        .border_color(color.opacity(0.42))
        .bg(color.opacity(if mode == ListMode::Ipset { 0.20 } else { 0.22 }))
        .shadow_sm()
        .left(px(2. + 170. * progress))
        .into_any_element()
}

fn mode_item(
    id: &'static str,
    label: impl Into<SharedString>,
    selected: bool,
    color: Rgba,
    click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    div()
        .id(id)
        .relative()
        .flex_1()
        .h(px(30.))
        .px_3()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.))
        .text_size(px(12.))
        .line_height(px(14.4))
        .text_color(if selected {
            color
        } else {
            foreground().opacity(0.8)
        })
        .cursor_pointer()
        .on_click(click)
        .child(label.into())
}

#[cfg(test)]
mod tests {
    use super::critical_download_fraction;

    #[test]
    fn critical_download_progress_handles_indeterminate_and_caps_complete() {
        assert_eq!(critical_download_fraction(0, 0), None);
        assert_eq!(critical_download_fraction(5, 10), Some(0.5));
        assert_eq!(critical_download_fraction(15, 10), Some(1.0));
    }
}
