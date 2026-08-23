use super::*;

pub(super) fn module_detail_page(
    title: impl Into<SharedString>,
    description: impl Into<SharedString>,
    action: impl IntoElement,
    back: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    content: impl IntoElement,
    cx: &App,
) -> AnyElement {
    let title = title.into();
    let description = description.into();
    div()
        .size_full()
        .child(SmoothVerticalScroll::new(
            SharedString::from(format!("module-{title}")),
            div()
                .min_h_full()
                .px_6()
                .pt(PAGE_TOP_PADDING)
                .pb_6()
                .flex()
                .flex_col()
                .gap(PAGE_HEADER_GAP)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_4()
                        .child(
                            div()
                                .min_w_0()
                                .flex()
                                .items_center()
                                .gap_4()
                                .child(
                                    crate::ui::components::button::IconButton::new(
                                        SharedString::from(format!("back-{title}")),
                                        "icons/arrow-left.svg",
                                        cx,
                                    )
                                    .ghost()
                                    .small()
                                    .on_click(back),
                                )
                                .child(
                                    div()
                                        .min_w_0()
                                        .child(
                                            div()
                                                .text_2xl()
                                                .line_height(px(32.))
                                                .font_weight(FontWeight::MEDIUM)
                                                .child(title),
                                        )
                                        .child(
                                            div()
                                                .mt_1()
                                                .text_sm()
                                                .line_height(px(20.))
                                                .text_color(muted_foreground())
                                                .child(description),
                                        ),
                                ),
                        )
                        .child(action),
                )
                .child(content),
        ))
        .into_any_element()
}

pub(super) fn module_power_button(
    id: &'static str,
    enabled: bool,
    cx: &App,
    click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let state_key = SharedString::from(format!("{id}-active-state"));
    let hover_key = SharedString::from(format!("{id}-hover"));
    let active_progress =
        crate::ui::foundation::hover_motion::state_progress(&state_key, enabled, cx);
    let hover_progress = crate::ui::foundation::hover_motion::progress(&hover_key, cx);
    let sk = state_key.clone();
    let hk = hover_key.clone();

    let target_bg = mix_color(accent(), danger(), active_progress);
    let target_border = mix_color(accent(), danger(), active_progress);
    let target_fg = mix_color(accent_foreground(), rgba(0xffffffff), active_progress);

    let bg_color = mix_color(
        target_bg,
        mix_color(target_bg, background(), 0.12),
        hover_progress,
    );

    let label = if enabled {
        t!("modules.btn_disable")
    } else {
        t!("modules.btn_enable")
    };

    div()
        .id(id)
        .h(crate::ui::foundation::control_style::CONTROL_HEIGHT)
        .px(px(14.))
        .flex()
        .items_center()
        .gap_2()
        .rounded_md()
        .border_1()
        .border_color(target_border)
        .bg(bg_color)
        .text_color(target_fg)
        .text_size(px(13.))
        .font_weight(FontWeight::MEDIUM)
        .cursor_pointer()
        .on_hover(move |hovered, window, cx| {
            crate::ui::foundation::hover_motion::set_hovered(hk.clone(), *hovered, window, cx);
        })
        .on_click(move |event, window, cx| {
            crate::ui::foundation::hover_motion::set_active(sk.clone(), !enabled, window, cx);
            click(event, window, cx);
        })
        .child(svg().path("icons/power.svg").size_4().text_color(target_fg))
        .child(label)
        .into_any_element()
}

pub(super) fn primary_button(
    id: impl Into<ElementId>,
    icon: &'static str,
    label: impl Into<SharedString>,
    click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> AnyElement {
    crate::ui::components::button::Button::new(id, label, cx)
        .primary()
        .icon_prefix(icon)
        .on_click(click)
        .into_element()
}

pub(super) fn outline_button(
    id: impl Into<ElementId>,
    icon: &'static str,
    label: impl Into<SharedString>,
    click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> AnyElement {
    crate::ui::components::button::Button::new(id, label, cx)
        .secondary()
        .icon_prefix(icon)
        .on_click(click)
        .into_element()
}

pub(super) fn ping_button(
    id: &'static str,
    checking: bool,
    click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> AnyElement {
    let label = if checking {
        t!("modules.checking_latency")
    } else {
        t!("modules.btn_check_latency")
    };

    crate::ui::components::button::Button::new(id, label, cx)
        .secondary()
        .loading(checking)
        .icon_prefix("icons/refresh-cw.svg")
        .on_click(click)
        .into_element()
}

#[allow(clippy::too_many_arguments)]
pub(super) fn dns_provider_card(
    id: &'static str,
    name: &'static str,
    url: &'static str,
    selected: bool,
    latency: Option<Option<u128>>,
    checking: bool,
    state: Entity<crate::app_state::AppState>,
    cx: &App,
) -> AnyElement {
    let latency_element = if checking {
        Some(
            crate::ui::components::badge::Badge::new("...")
                .accent()
                .monospace()
                .pulse(format!("dns-ping-pulse-{id}"))
                .into_any_element(),
        )
    } else if let Some(latency_val) = latency {
        let (variant, text) = match latency_val {
            Some(ms) => {
                let variant = if ms < 60 {
                    crate::ui::components::badge::BadgeVariant::Success
                } else if ms <= 150 {
                    crate::ui::components::badge::BadgeVariant::Accent
                } else {
                    crate::ui::components::badge::BadgeVariant::Warning
                };
                (variant, format!("{ms} мс"))
            }
            None => (
                crate::ui::components::badge::BadgeVariant::Destructive,
                "н/д".to_string(),
            ),
        };
        Some(
            crate::ui::components::badge::Badge::new(text)
                .variant(variant)
                .monospace()
                .fade_in(format!("dns-latency-fade-{id}"))
                .into_any_element(),
        )
    } else {
        None
    };

    let variant = if selected {
        crate::ui::components::card::CardVariant::Success
    } else {
        crate::ui::components::card::CardVariant::Interactive
    };

    crate::ui::components::card::Card::interactive(format!("dns-{id}"), cx)
        .variant(variant)
        .rounded_lg()
        .min_h(px(72.))
        .on_click(move |_, _, cx| {
            state.update(cx, |state, cx| {
                state.set_dns_preset(id, cx);
            })
        })
        .child(
            div()
                .p_4()
                .flex()
                .items_center()
                .justify_between()
                .gap_3()
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(div().text_sm().font_weight(FontWeight::MEDIUM).child(name))
                                .children(latency_element),
                        )
                        .child(
                            div()
                                .mt_1()
                                .text_xs()
                                .font_family("IBM Plex Mono")
                                .text_color(muted_foreground())
                                .child(url),
                        ),
                )
                .when(selected, |card| {
                    card.child(
                        svg()
                            .path("icons/check.svg")
                            .size_4()
                            .text_color(success())
                            .with_animation(
                                SharedString::from(format!("dns-check-fade-{id}")),
                                Animation::new(std::time::Duration::from_millis(200)),
                                |icon, delta| icon.opacity(delta),
                            ),
                    )
                }),
        )
        .into_element()
}

pub(super) fn input_control(
    state: &Entity<crate::ui::components::text_input::TextInputState>,
    width: Pixels,
) -> Div {
    div()
        .w(width)
        .h(crate::ui::foundation::control_style::CONTROL_HEIGHT)
        .px_3()
        .flex_none()
        .rounded_md()
        .border_1()
        .border_color(border())
        .bg(input().opacity(0.3))
        .child(TextInput::new(state))
}

pub(super) fn secret_control(
    secret: &Entity<crate::ui::components::text_input::TextInputState>,
    port: Entity<crate::ui::components::text_input::TextInputState>,
    state: Entity<crate::app_state::AppState>,
    cx: &App,
) -> AnyElement {
    let secret_state = secret.clone();
    let refresh_btn = crate::ui::components::button::IconButton::new(
        "generate-tg-secret-btn",
        "icons/refresh-cw.svg",
        cx,
    )
    .ghost()
    .small()
    .on_click(move |_, _, cx| {
        let value = uuid::Uuid::new_v4().simple().to_string();
        secret_state.update(cx, |input, cx| input.set_value(value.clone(), cx));
        state.update(cx, |state, cx| {
            state.set_tg_settings(port.read(cx).value().to_owned(), value, cx)
        });
    });

    div()
        .relative()
        .w(px(384.))
        .child(input_control(secret, px(384.)).pr(px(36.)))
        .child(
            div()
                .absolute()
                .right(px(4.))
                .top(px(4.))
                .child(refresh_btn),
        )
        .into_any_element()
}

pub(super) fn copy_card(label: impl Into<SharedString>, value: String, cx: &App) -> AnyElement {
    let label = label.into();
    let copy = value.clone();
    let btn_id = SharedString::from(format!("copy-card-btn-{label}"));

    let copy_btn = crate::ui::components::button::IconButton::new(btn_id, "icons/copy.svg", cx)
        .ghost()
        .small()
        .on_click(move |_, _, cx| cx.write_to_clipboard(ClipboardItem::new_string(copy.clone())));

    crate::ui::components::card::Card::new()
        .variant(crate::ui::components::card::CardVariant::Muted)
        .rounded_lg()
        .min_h(px(72.))
        .child(
            div()
                .p_3()
                .flex()
                .flex_col()
                .child(div().text_xs().text_color(muted_foreground()).child(label))
                .child(
                    div()
                        .mt_2()
                        .pt_2()
                        .border_t_1()
                        .border_color(border().opacity(0.6))
                        .flex()
                        .items_end()
                        .justify_between()
                        .gap_3()
                        .child(
                            div()
                                .min_w_0()
                                .text_sm()
                                .font_family("IBM Plex Mono")
                                .child(value),
                        )
                        .child(copy_btn),
                ),
        )
        .into_element()
}
