use super::*;
use crate::ui::components::card::{module_body, module_card, module_header};

impl AppView {
    pub(crate) fn about_page(&self, cx: &mut Context<Self>) -> AnyElement {
        let project = module_card(
            module_header(
                ("icons/package.svg", colors::yellow()),
                "Zapret Interactive",
                "Desktop GUI для zapret-win-bundle с управлением стратегиями, фильтрами и плейсхолдерами",
                None,
                true,
            ),
            Some(
                module_body()
                    .grid()
                    .grid_cols(2)
                    .gap_3()
                    .child(meta_card(
                        "icons/package.svg",
                        t!("about.version"),
                        env!("CARGO_PKG_VERSION"),
                        true,
                    ))
                    .child(meta_card(
                        "icons/user-round.svg",
                        t!("about.developer"),
                        "Noktomezo",
                        false,
                    )),
            ),
        );
        let links: [(SharedString, SharedString, &'static str, &'static str); 8] = [
            (
                t!("about.sources").into(),
                "Noktomezo/ZapretInteractive".into(),
                "https://github.com/Noktomezo/ZapretInteractive",
                "icons/external-link.svg",
            ),
            (
                t!("about.releases").into(),
                t!("about.releases_desc").into(),
                "https://github.com/Noktomezo/ZapretInteractive/releases",
                "icons/download.svg",
            ),
            (
                t!("about.license").into(),
                t!("about.license_desc").into(),
                "https://github.com/Noktomezo/ZapretInteractive/blob/main/LICENSE",
                "icons/shield.svg",
            ),
            (
                "zapret".into(),
                "Базовый DPI-bypass toolkit".into(),
                "https://github.com/bol-van/zapret",
                "icons/external-link.svg",
            ),
            (
                "zapret-win-bundle".into(),
                "Windows bundle и служебные файлы".into(),
                "https://github.com/bol-van/zapret-win-bundle",
                "icons/external-link.svg",
            ),
            (
                "Flexoki".into(),
                "Используемая тема и палитра интерфейса".into(),
                "https://github.com/kepano/flexoki",
                "icons/external-link.svg",
            ),
            (
                "dnscrypt-proxy".into(),
                "Основа DNS-модуля и DoH-прокси".into(),
                "https://github.com/DNSCrypt/dnscrypt-proxy",
                "icons/external-link.svg",
            ),
            (
                "tg-ws-proxy-rs".into(),
                "Основа Telegram WS Proxy модуля".into(),
                "https://github.com/valnesfjord/tg-ws-proxy-rs",
                "icons/external-link.svg",
            ),
        ];
        let links = links.into_iter().map(|(title, value, url, icon)| {
            about_link(title, value, url, icon, self.state.clone(), cx)
        });
        let metadata = module_card(
            module_header(
                ("icons/external-link.svg", colors::blue()),
                t!("about.metadata_title"),
                t!("about.metadata_desc"),
                None,
                true,
            ),
            Some(module_body().grid().grid_cols(2).gap_3().children(links)),
        );
        page(
            t!("about.title"),
            div()
                .flex()
                .flex_col()
                .gap_6()
                .child(project)
                .child(metadata),
        )
    }
}

fn meta_card(
    icon: &'static str,
    label: impl Into<SharedString>,
    value: impl Into<SharedString>,
    version: bool,
) -> AnyElement {
    let label = label.into();
    let value = value.into();
    crate::ui::components::card::Card::new()
        .variant(crate::ui::components::card::CardVariant::Muted)
        .rounded_lg()
        .min_h(px(84.))
        .child(
            div()
                .p_4()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .text_xs()
                        .text_color(muted_foreground())
                        .child(svg().path(icon).size_4().text_color(muted_foreground()))
                        .child(label.to_uppercase())
                        .when(version, |label| {
                            label.child(
                                crate::ui::components::badge::Badge::new(t!(
                                    "settings.badge_latest"
                                ))
                                .success(),
                            )
                        }),
                )
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::MEDIUM)
                        .when(version, |value| value.font_family("IBM Plex Mono"))
                        .child(value),
                ),
        )
        .into_element()
}

fn about_link(
    title: impl Into<SharedString>,
    value: impl Into<SharedString>,
    url: &'static str,
    icon: &'static str,
    state: Entity<crate::app_state::AppState>,
    cx: &App,
) -> AnyElement {
    let title = title.into();
    let value = value.into();
    let card_id = SharedString::from(format!("about-{title}"));

    crate::ui::components::card::Card::interactive(card_id, cx)
        .variant(crate::ui::components::card::CardVariant::Muted)
        .rounded_lg()
        .min_h(px(72.))
        .on_click(move |_, _, cx| state.update(cx, |state, cx| state.open_external(url, cx)))
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
                                .truncate()
                                .text_sm()
                                .font_weight(FontWeight::MEDIUM)
                                .child(title),
                        )
                        .child(
                            div()
                                .mt_1()
                                .text_xs()
                                .text_color(muted_foreground())
                                .truncate()
                                .child(value),
                        ),
                )
                .child(
                    div()
                        .size(px(32.))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(8.))
                        .border_1()
                        .border_color(border().opacity(0.6))
                        .bg(background().opacity(0.7))
                        .child(svg().path(icon).size_4().text_color(muted_foreground())),
                ),
        )
        .into_element()
}
