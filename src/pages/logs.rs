use super::module_detail::outline_button;
use super::*;
use chrono::{DateTime, Local};

impl AppView {
    pub(crate) fn logs_page(&self, cx: &mut Context<Self>) -> AnyElement {
        let rows = self
            .state
            .read(cx)
            .logs
            .iter()
            .map(|entry| {
                div()
                    .text_size(px(12.))
                    .line_height(px(16.))
                    .font_family("IBM Plex Mono")
                    .child(
                        div()
                            .w_full()
                            .min_w_0()
                            .flex()
                            .items_start()
                            .gap_1()
                            .child(
                                div()
                                    .flex_none()
                                    .text_color(muted_foreground())
                                    .child(format!("[{}]", format_timestamp(entry.timestamp))),
                            )
                            .child(
                                div()
                                    .min_w_0()
                                    .flex_1()
                                    .whitespace_normal()
                                    .child(entry.message.clone()),
                            ),
                    )
            })
            .collect::<Vec<_>>();
        let has_logs = !rows.is_empty();
        let state = self.state.clone();

        div()
            .size_full()
            .min_h_0()
            .px_6()
            .pt(PAGE_TOP_PADDING)
            .pb_6()
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_4()
                    .child(
                        div()
                            .child(
                                div()
                                    .text_2xl()
                                    .line_height(px(32.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(t!("logs.title")),
                            )
                            .child(
                                div()
                                    .mt_1()
                                    .text_sm()
                                    .line_height(px(20.))
                                    .text_color(muted_foreground())
                                    .child(t!("logs.desc")),
                            ),
                    )
                    .child(outline_button(
                        "clear-logs",
                        "icons/brush-cleaning.svg",
                        t!("logs.btn_clear"),
                        move |_, _, cx| state.update(cx, |state, cx| state.clear_logs(cx)),
                        cx,
                    )),
            )
            .child(
                div()
                    .relative()
                    .mt(PAGE_HEADER_GAP)
                    .min_h_0()
                    .flex_1()
                    .overflow_hidden()
                    .rounded(px(12.))
                    .border_1()
                    .border_color(border())
                    .bg(card_color())
                    .when(has_logs, |container| {
                        container.child(SmoothVerticalScroll::new(
                            "logs-scroll",
                            div().p_4().flex().flex_col().gap_1().children(rows),
                        ))
                    })
                    .when(!has_logs, |container| {
                        container
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_sm()
                            .text_color(muted_foreground())
                            .child(t!("logs.empty"))
                    }),
            )
            .into_any_element()
    }
}

fn format_timestamp(timestamp: std::time::SystemTime) -> String {
    let timestamp: DateTime<Local> = timestamp.into();
    timestamp.format("%d.%m.%Y, %H:%M:%S").to_string()
}
