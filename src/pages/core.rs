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
