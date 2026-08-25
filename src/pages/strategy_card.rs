use super::*;
use crate::app_state::AppState;
use crate::domain::Strategy;
use crate::ui::components::card::{Card, CardVariant};

pub fn strategy_modified(strategy: &Strategy, builtin: Option<&Strategy>) -> bool {
    let Some(builtin) = builtin else {
        return false;
    };
    strategy.system && (strategy.name != builtin.name || strategy.content != builtin.content)
}

pub(super) fn strategy_card(
    category_id: String,
    strategy: Strategy,
    builtin: Option<Strategy>,
    state: Entity<AppState>,
    edit: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> AnyElement {
    let active = strategy.active;
    let modified = strategy_modified(&strategy, builtin.as_ref());
    let name_text = strategy.name.clone();
    let strategy_id = strategy.id.clone();
    let activate_category = category_id.clone();
    let activate_id = strategy.id.clone();
    let clear_category = category_id.clone();
    let clear_id = strategy.id.clone();
    let restore_category = category_id.clone();
    let restore_id = strategy.id.clone();
    let delete_category = category_id;
    let delete_id = strategy.id.clone();
    let activate_state = state.clone();
    let clear_state = state.clone();
    let restore_state = state.clone();
    let delete_state = state;

    Card::new()
        .variant(if active {
            CardVariant::Success
        } else {
            CardVariant::Default
        })
        .child(
            div()
                .p_4()
                .flex()
                .flex_col()
                .gap_3()
                .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_3()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .text_base()
                                .line_height(px(20.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(foreground())
                                .child(name_text),
                        )
                        .child(source_kind_badge(
                            SharedString::from(format!("badge-strat-{strategy_id}")),
                            strategy.system,
                            px(13.),
                        ))
                        .when(strategy.system && modified, |name| {
                            let restore_category = restore_category.clone();
                            let restore_id = restore_id.clone();
                            let restore_state = restore_state.clone();
                            let restore_id_str = SharedString::from(format!("restore-inline-{restore_id}"));
                            name.child(restore_badge(
                                restore_id_str,
                                None,
                                move |_, _, cx| {
                                    restore_state.update(cx, |state, cx| {
                                        state.restore_strategy(
                                            &restore_category,
                                            &restore_id,
                                            cx,
                                        );
                                    });
                                },
                            ))
                        }),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .when(!active, |buttons| {
                            buttons.child(icon_button(
                                SharedString::from(format!("activate-strategy-{activate_id}")),
                                "icons/check.svg",
                                IconButtonVariant::Outline,
                                t!("common.apply"),
                                move |_, _, cx| {
                                    activate_state.update(cx, |state, cx| {
                                        state.select_strategy(&activate_category, &activate_id, cx)
                                    })
                                },
                                cx,
                            ))
                        })
                        .child(icon_button(
                            SharedString::from(format!("edit-strategy-{strategy_id}")),
                            "icons/pencil.svg",
                            IconButtonVariant::Outline,
                            t!("common.edit"),
                            edit,
                            cx,
                        ))
                        .when(modified, |buttons| {
                            buttons.child(icon_button(
                                SharedString::from(format!("restore-strategy-{restore_id}")),
                                "icons/rotate-ccw.svg",
                                IconButtonVariant::Warning,
                                t!("strategies.restore_tooltip"),
                                move |_, _, cx| {
                                    restore_state.update(cx, |state, cx| {
                                        state.restore_strategy(&restore_category, &restore_id, cx)
                                    })
                                },
                                cx,
                            ))
                        })
                        .when(active, |buttons| {
                            buttons.child(icon_button(
                                SharedString::from(format!("clear-strategy-{clear_id}")),
                                "icons/rotate-ccw.svg",
                                IconButtonVariant::Warning,
                                t!("strategies.btn_clear_active"),
                                move |_, _, cx| {
                                    clear_state.update(cx, |state, cx| {
                                        state.select_strategy(&clear_category, &clear_id, cx)
                                    })
                                },
                                cx,
                            ))
                        })
                        .child(icon_button(
                            SharedString::from(format!("delete-strategy-{delete_id}")),
                            "icons/trash-2.svg",
                            IconButtonVariant::Destructive,
                            t!("dialog.delete_strategy"),
                            {
                                let strategy_name = strategy.name.clone();
                                move |_, _, cx| {
                                    delete_state.update(cx, |state, cx| {
                                        state.set_confirm(
                                            Some(crate::ui::components::confirm_dialog::ConfirmTarget::DeleteStrategy {
                                                category_id: delete_category.clone(),
                                                strategy_id: delete_id.clone(),
                                                name: strategy_name.clone(),
                                            }),
                                            cx,
                                        );
                                    })
                                }
                            },
                            cx,
                        )),
                ),
        )
        .child(
            div()
                .w_full()
                .p_3()
                .rounded_md()
                .border_1()
                .border_color(if active {
                    success().opacity(0.3)
                } else {
                    border().opacity(0.8)
                })
                .bg(if active {
                    mix_color(background(), success(), 0.10)
                } else {
                    background().opacity(0.84)
                })
                .text_xs()
                .line_height(px(16.))
                .font_family("IBM Plex Mono")
                .text_color(muted_foreground())
                .child(strategy.content),
        ))
        .into_element()
}
