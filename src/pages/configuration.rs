use super::module_detail::*;
use super::*;
use crate::ui::components::card::{module_body, module_card, module_header, module_row};
use crate::ui::components::dropdown::dropdown;

impl AppView {
    pub(crate) fn dns_page(&self, cx: &mut Context<Self>) -> AnyElement {
        let (config, latencies, checking) = {
            let state = self.state.read(cx);
            (
                state.config.clone(),
                state.dns_latencies.clone(),
                state.dns_checking,
            )
        };
        let toggle = module_power_button("toggle-dns-module", config.dns_module_enabled, cx, {
            let state = self.state.clone();
            move |_, _, cx| {
                state.update(cx, |state, cx| {
                    state.set_dns_module(!state.config.dns_module_enabled, cx);
                })
            }
        });
        let bootstrap_control = dropdown("dns-bootstrap", &self.dns_dropdown, cx);
        let multiqueue = switch("dns-accelerator", config.dns_accelerator_enabled, cx).on_toggle({
            let state = self.state.clone();
            move |_, _, cx| {
                state.update(cx, |state, cx| {
                    state.set_dns_multiqueue(!state.config.dns_accelerator_enabled, cx);
                })
            }
        });
        let parameters = module_card(
            module_header(
                ("icons/shield-check.svg", colors::green()),
                t!("modules.dns_parameters"),
                t!("modules.dns_parameters_desc"),
                None,
                true,
            ),
            Some(
                module_body()
                    .child(module_row(
                        t!("modules.dns_bootstrap"),
                        t!("modules.dns_bootstrap_desc"),
                        bootstrap_control,
                    ))
                    .child(module_row(
                        t!("modules.dns_accelerator"),
                        t!("modules.dns_accelerator_desc"),
                        multiqueue,
                    )),
            ),
        );
        let providers = PRESETS.into_iter().map(|(id, name, url)| {
            dns_provider_card(
                id,
                name,
                url,
                config.dns_preset_id == id,
                config.dns_accelerator_enabled,
                latencies.get(id).copied(),
                checking,
                self.state.clone(),
                cx,
            )
        });
        let ping = ping_button(
            "check-dns-latency",
            checking,
            {
                let state = self.state.clone();
                move |_, _, cx| state.update(cx, |state, cx| state.check_dns_latencies(cx))
            },
            cx,
        );
        let provider_card = module_card(
            module_header(
                ("icons/globe.svg", colors::blue()),
                t!("modules.dns_servers"),
                t!("modules.dns_servers_desc"),
                Some(ping),
                true,
            ),
            Some(div().p_4().grid().grid_cols(2).gap_3().children(providers)),
        );
        module_detail_page(
            t!("modules.dns_title"),
            t!("modules.dns_desc"),
            toggle,
            cx.listener(|this, _, _, cx| this.navigate(Route::Modules, cx)),
            div()
                .flex()
                .flex_col()
                .gap_6()
                .child(parameters)
                .child(provider_card),
            cx,
        )
    }

    pub(crate) fn tg_page(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let config = self.state.read(cx).config.clone();
        let proxy_pid = self.state.read(cx).tg_proxy_pid();
        let toggle =
            module_power_button("toggle-tg-module", config.tg_ws_proxy_module_enabled, cx, {
                let state = self.state.clone();
                move |_, _, cx| {
                    state.update(cx, |state, cx| {
                        state.set_tg_proxy_module(!state.config.tg_ws_proxy_module_enabled, cx);
                    })
                }
            });
        let parameters = module_card(
            module_header(
                ("icons/shield-check.svg", colors::cyan()),
                t!("modules.tg_proxy_parameters"),
                t!("modules.tg_proxy_parameters_desc"),
                None,
                true,
            ),
            Some(
                module_body()
                    .child(module_row(
                        t!("modules.tg_proxy_port"),
                        t!("modules.tg_proxy_port_desc"),
                        input_control(&self.tg_port_input, px(128.)),
                    ))
                    .child(module_row(
                        t!("modules.tg_proxy_secret"),
                        t!("modules.tg_proxy_secret_desc"),
                        secret_control(
                            &self.tg_secret_input,
                            self.tg_port_input.clone(),
                            self.state.clone(),
                            cx,
                        ),
                    )),
            ),
        );
        let link = format!(
            "tg://proxy?server=127.0.0.1&port={}&secret=dd{}",
            config.tg_ws_proxy_port, config.tg_ws_proxy_secret
        );
        let chevron_id: SharedString = "tg-info-chevron".into();
        let chevron_progress = disclosure_progress(&chevron_id, self.tg_info_expanded, cx);
        let connection_action = div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                DisclosureChevron::new(chevron_id, self.tg_info_expanded, cx).on_click(
                    cx.listener(move |this, _, _, cx| {
                        this.tg_info_expanded = !this.tg_info_expanded;
                        cx.notify();
                    }),
                ),
            )
            .child(primary_button(
                "open-tg-link",
                "icons/send.svg",
                t!("modules.btn_copy_tg_link"),
                {
                    let state = self.state.clone();
                    let link = link.clone();
                    move |_, _, cx| state.update(cx, |state, cx| state.open_external(&link, cx))
                },
                cx,
            ));
        let connection_body = (chevron_progress > 0.001).then(|| {
            div()
                .overflow_hidden()
                .opacity(chevron_progress)
                .p_4()
                .flex()
                .flex_col()
                .gap_4()
                .child(
                    div()
                        .grid()
                        .grid_cols(3)
                        .gap_3()
                        .child(copy_card("Хост", "127.0.0.1".into(), cx))
                        .child(copy_card(
                            t!("modules.tg_proxy_port"),
                            config.tg_ws_proxy_port.to_string(),
                            cx,
                        ))
                        .child(copy_card(
                            "PID",
                            proxy_pid.map_or_else(|| "—".into(), |pid| pid.to_string()),
                            cx,
                        )),
                )
                .child(copy_card(t!("modules.tg_proxy_secret"), link.clone(), cx))
        });
        let connection = module_card(
            module_header(
                ("icons/send.svg", colors::blue()),
                t!("modules.tg_instructions"),
                t!("modules.tg_instructions_desc"),
                Some(connection_action.into_any_element()),
                true,
            ),
            connection_body,
        );
        module_detail_page(
            t!("modules.tg_proxy_title"),
            t!("modules.tg_proxy_desc"),
            toggle,
            cx.listener(|this, _, _, cx| this.navigate(Route::Modules, cx)),
            div()
                .flex()
                .flex_col()
                .gap_6()
                .child(parameters)
                .child(connection),
            cx,
        )
    }
}
