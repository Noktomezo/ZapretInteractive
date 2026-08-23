use super::*;
use crate::ui::components::card::{
    module_body, module_card, module_header, module_header_custom, module_row,
};
use crate::ui::components::dropdown::dropdown;

impl AppView {
    pub(crate) fn settings_page(&self, cx: &mut Context<Self>) -> AnyElement {
        let (config, autostart) = {
            let state = self.state.read(cx);
            (state.config.clone(), state.autostart_enabled)
        };

        page(
            t!("settings.title"),
            div()
                .flex()
                .flex_col()
                .gap_6()
                .child(appearance_card(
                    &config,
                    &self.theme_dropdown,
                    &self.language_dropdown,
                    self.state.clone(),
                    cx,
                ))
                .child(updates_card(&config, self.state.clone(), cx))
                .child(behavior_card(
                    &config,
                    autostart,
                    &self.discord_dropdown,
                    self.state.clone(),
                    cx,
                ))
                .child(ports_card(&self.tcp_input, &self.udp_input))
                .child(reset_card(self.state.clone(), cx)),
        )
    }
}

fn appearance_card(
    config: &crate::domain::AppConfig,
    theme_dropdown: &Entity<crate::ui::components::dropdown::DropdownState>,
    language_dropdown: &Entity<crate::ui::components::dropdown::DropdownState>,
    state: Entity<crate::app_state::AppState>,
    cx: &App,
) -> AnyElement {
    let state_acrylic = state;
    let acrylic_enabled = config.acrylic_material;
    let body = module_body()
        .child(module_row(
            t!("settings.theme_title"),
            t!("settings.theme_desc"),
            dropdown("theme-select", theme_dropdown, cx),
        ))
        .child(module_row(
            t!("settings.language_title"),
            t!("settings.language_desc"),
            dropdown("language-select", language_dropdown, cx),
        ))
        .child(module_row(
            t!("settings.acrylic_title"),
            t!("settings.acrylic_desc"),
            switch("acrylic-material", acrylic_enabled, cx).on_toggle(move |_, _, cx| {
                state_acrylic.update(cx, |s, cx| s.set_acrylic_material(!acrylic_enabled, cx));
            }),
        ));

    module_card(
        module_header(
            ("icons/palette.svg", colors::purple()),
            t!("settings.appearance_title"),
            t!("settings.appearance_desc"),
            None,
            true,
        ),
        Some(body),
    )
    .into_any_element()
}

fn updates_card(
    config: &crate::domain::AppConfig,
    state: Entity<crate::app_state::AppState>,
    cx: &App,
) -> AnyElement {
    let (
        app_update,
        app_update_checked,
        app_update_error,
        checking_app_update,
        is_updating,
        update_download_progress,
    ) = {
        let app_state = state.read(cx);
        (
            app_state.app_update.clone(),
            app_state.app_update_checked,
            app_state.app_update_error.clone(),
            app_state.checking_app_update,
            app_state.is_updating,
            app_state.update_download_progress,
        )
    };
    let mut body = module_body();

    if let Some(error) = app_update_error.as_deref() {
        body = body.child(
            div()
                .text_xs()
                .text_color(colors::destructive())
                .child(t!("settings.update_failed", error = error)),
        );
    }

    let state_auto = state.clone();
    let auto_updates_enabled = config.app_auto_updates_enabled;
    body = body.child(module_row(
        t!("settings.auto_updates"),
        t!("settings.auto_updates_desc"),
        switch("app-auto-updates", auto_updates_enabled, cx).on_toggle(move |_, _, cx| {
            state_auto.update(cx, |s, cx| {
                s.set_app_auto_updates(!auto_updates_enabled, cx)
            });
        }),
    ));

    let state_prompts = state.clone();
    let file_prompts_enabled = config.core_file_update_prompts_enabled;
    body = body.child(module_row(
        t!("settings.file_updates"),
        t!("settings.file_updates_desc"),
        switch("core-file-update-prompts", file_prompts_enabled, cx).on_toggle(move |_, _, cx| {
            state_prompts.update(cx, |s, cx| {
                s.set_core_file_update_prompts(!file_prompts_enabled, cx)
            });
        }),
    ));

    if let Some(action) = render_file_update_action(state.clone(), cx) {
        body = body.child(action);
    }

    let current_version = env!("CARGO_PKG_VERSION");

    let version_badge = crate::ui::components::badge::Badge::new(format!("v{current_version}"))
        .neutral()
        .monospace();

    let status_badge = if checking_app_update {
        Some(crate::ui::components::badge::loading_badge(t!(
            "settings.checking_version"
        )))
    } else if is_updating {
        Some(update_download_progress.map_or_else(
            || {
                crate::ui::components::badge::loading_badge(t!(
                    "settings.update_downloading_indeterminate"
                ))
            },
            |progress| {
                let percent = (progress.clamp(0.0, 1.0) * 100.0).round() as u32;
                crate::ui::components::badge::progress_badge(
                    t!("settings.update_downloading", percent = percent),
                    progress,
                )
            },
        ))
    } else if let Some(update) = app_update.as_ref() {
        Some(
            crate::ui::components::badge::Badge::new(format!(
                "{} · v{}",
                t!("settings.badge_update_available"),
                update.new_version.trim_start_matches('v')
            ))
            .orange()
            .pulse("settings-update-available-pulse")
            .into_any_element(),
        )
    } else if app_update_checked {
        Some(
            crate::ui::components::badge::Badge::new(t!("settings.badge_latest"))
                .success()
                .into_any_element(),
        )
    } else {
        None
    };

    let title_element = div()
        .flex()
        .items_center()
        .gap_2()
        .child(
            div()
                .text_sm()
                .line_height(px(20.))
                .font_weight(FontWeight::MEDIUM)
                .child(t!("settings.updates_title")),
        )
        .child(version_badge)
        .children(status_badge);

    let actions = update_actions(
        app_update.is_some(),
        checking_app_update,
        is_updating,
        state,
        cx,
    );

    module_card(
        module_header_custom(
            ("icons/cloud-download.svg", colors::green()),
            title_element,
            t!("settings.updates_desc"),
            Some(actions),
            true,
        ),
        Some(body),
    )
    .into_any_element()
}

fn update_actions(
    has_update: bool,
    checking_app_update: bool,
    is_updating: bool,
    state: Entity<crate::app_state::AppState>,
    cx: &App,
) -> AnyElement {
    let mut actions = div().flex_none().flex().items_center().gap_2();
    if has_update && !checking_app_update && !is_updating {
        let update_state = state.clone();
        actions = actions.child(
            crate::ui::components::button::Button::new(
                "btn-apply-update",
                t!("settings.btn_update_and_restart"),
                cx,
            )
            .orange()
            .icon_prefix("icons/cloud-download.svg")
            .on_click(move |_, _, cx| {
                update_state.update(cx, |state, cx| state.trigger_app_update(cx));
            }),
        );
    }

    actions
        .child(
            crate::ui::components::button::IconButton::new(
                "btn-check-app-update",
                "icons/refresh-cw.svg",
                cx,
            )
            .outline()
            .loading(checking_app_update)
            .disabled(is_updating)
            .tooltip(t!("settings.btn_check_version"))
            .on_click(move |_, _, cx| {
                state.update(cx, |state, cx| state.check_app_updates(cx));
            }),
        )
        .into_any_element()
}

fn render_file_update_action(
    state: Entity<crate::app_state::AppState>,
    cx: &App,
) -> Option<AnyElement> {
    let app_state = state.read(cx);
    let downloading_files = app_state.download_progress.is_some();
    let needs_download = downloading_files
        || (!app_state.checking_files
            && (!app_state.health.binaries_ok
                || (app_state.config.core_file_update_prompts_enabled
                    && !app_state.health.available_updates.is_empty())));
    if !needs_download {
        return None;
    }
    let download_label = if let Some(progress) = &app_state.download_progress {
        t!(
            "home.downloading_files",
            current = progress.current,
            total = progress.total
        )
    } else if app_state.health.binaries_ok {
        t!("home.btn_update_critical_files")
    } else {
        t!("home.btn_restore_critical_files")
    };

    let state_download = state;

    Some(
        div()
            .pt_2()
            .child(
                crate::ui::components::button::Button::new(
                    "btn-download-files",
                    download_label,
                    cx,
                )
                .destructive()
                .small()
                .loading(downloading_files)
                .icon_prefix("icons/download.svg")
                .on_click(move |_, _, cx| {
                    state_download.update(cx, |state, cx| state.download_files(cx));
                }),
            )
            .into_any_element(),
    )
}

fn behavior_card(
    config: &crate::domain::AppConfig,
    autostart: bool,
    discord_dropdown: &Entity<crate::ui::components::dropdown::DropdownState>,
    state: Entity<crate::app_state::AppState>,
    cx: &App,
) -> AnyElement {
    let mut body = module_body();

    let state_auto = state.clone();
    body = body.child(module_row(
        t!("settings.autostart"),
        t!("settings.autostart_desc"),
        switch("autostart", autostart, cx).on_toggle(move |_, _, cx| {
            state_auto.update(cx, |s, cx| s.set_autostart(!autostart, cx));
        }),
    ));

    if autostart {
        let state_conn = state.clone();
        let conn_auto = config.connect_on_autostart;
        let state_tray = state.clone();
        let launch_tray = config.launch_to_tray;

        let autostart_subgroup = div()
            .ml(px(18.))
            .pl_4()
            .border_l_1()
            .border_color(border().opacity(0.6))
            .flex()
            .flex_col()
            .gap_3()
            .child(module_row(
                t!("settings.connect_on_autostart"),
                t!("settings.connect_on_autostart_desc"),
                switch("connect-on-autostart", conn_auto, cx).on_toggle(move |_, _, cx| {
                    state_conn.update(cx, |s, cx| s.set_connect_on_autostart(!conn_auto, cx));
                }),
            ))
            .child(module_row(
                t!("settings.launch_to_tray"),
                t!("settings.launch_to_tray_desc"),
                switch("launch-to-tray", launch_tray, cx).on_toggle(move |_, _, cx| {
                    state_tray.update(cx, |s, cx| s.set_launch_to_tray(!launch_tray, cx));
                }),
            ));
        body = body.child(autostart_subgroup);
    }

    let state_min = state.clone();
    let min_tray = config.minimize_to_tray;
    body = body.child(module_row(
        t!("settings.minimize_to_tray"),
        t!("settings.minimize_to_tray_desc"),
        switch("minimize-to-tray", min_tray, cx).on_toggle(move |_, _, cx| {
            state_min.update(cx, |s, cx| s.set_minimize_to_tray(!min_tray, cx));
        }),
    ));

    body = body.child(module_row(
        t!("settings.discord_rpc"),
        t!("settings.discord_rpc_desc"),
        dropdown("discord-presence", discord_dropdown, cx),
    ));

    module_card(
        module_header(
            ("icons/app-window.svg", colors::blue()),
            t!("settings.behavior_title"),
            t!("settings.behavior_desc"),
            None,
            true,
        ),
        Some(body),
    )
    .into_any_element()
}

fn ports_card(
    tcp_input: &Entity<crate::ui::components::text_input::TextInputState>,
    udp_input: &Entity<crate::ui::components::text_input::TextInputState>,
) -> AnyElement {
    module_card(
        module_header(
            ("icons/router.svg", colors::yellow()),
            t!("settings.ports_title"),
            t!("settings.ports_desc"),
            None,
            true,
        ),
        Some(
            module_body()
                .child(module_row(
                    t!("settings.tcp_ports"),
                    t!("settings.tcp_ports_desc"),
                    settings_input(tcp_input),
                ))
                .child(module_row(
                    t!("settings.udp_ports"),
                    t!("settings.udp_ports_desc"),
                    settings_input(udp_input),
                )),
        ),
    )
    .into_any_element()
}

fn reset_card(state: Entity<crate::app_state::AppState>, cx: &App) -> AnyElement {
    module_card(
        module_header(
            ("icons/rotate-ccw.svg", colors::destructive()),
            t!("settings.reset_title"),
            t!("settings.reset_desc"),
            Some(reset_button(state, cx)),
            false,
        ),
        None,
    )
    .into_any_element()
}

fn settings_input(state: &Entity<crate::ui::components::text_input::TextInputState>) -> AnyElement {
    crate::ui::components::form_field::FormInput::new(state)
        .width(px(176.))
        .into_any_element()
}

fn reset_button(state: Entity<crate::app_state::AppState>, cx: &App) -> AnyElement {
    crate::ui::components::button::Button::new("reset-config", t!("settings.btn_reset"), cx)
        .destructive()
        .icon_prefix("icons/rotate-ccw.svg")
        .on_click(move |_, _, cx| {
            state.update(cx, |state, cx| {
                state.set_confirm(
                    Some(crate::ui::components::confirm_dialog::ConfirmTarget::ResetConfig),
                    cx,
                )
            })
        })
        .into_element()
}
