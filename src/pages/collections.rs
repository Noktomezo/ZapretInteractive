use super::strategy_card::strategy_card;
use super::*;
use crate::ui::components::confirm_dialog::ConfirmTarget;
use crate::ui::components::cursor_tooltip;
use crate::ui::components::smooth_scroll::{PageScrollbar, SmoothListScroll};

impl AppView {
    pub(crate) fn category_page(
        &mut self,
        category_id: &str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let (category, builtin) = {
            let app_state = self.state.read(cx);
            (
                app_state
                    .config
                    .categories
                    .iter()
                    .find(|item| item.id == category_id)
                    .cloned(),
                app_state
                    .builtin
                    .categories
                    .iter()
                    .find(|item| item.id == category_id)
                    .cloned(),
            )
        };
        let Some(category) = category else {
            return page("Стратегии", div().child("Категория не найдена"));
        };

        let modified = builtin
            .as_ref()
            .is_some_and(|builtin| category_modified(&category, builtin));
        let active_strategy = category
            .strategies
            .iter()
            .enumerate()
            .find(|(_, strategy)| strategy.active)
            .map(|(index, strategy)| (index + 1, strategy.name.clone()));
        let page_category_id = category.id.clone();
        let strategies = category.strategies.clone();

        let total_items = if strategies.is_empty() {
            2
        } else {
            1 + strategies.len()
        };
        let list_state = self.category_strategies_list_state.clone();
        let category_changed = self.current_viewed_category.as_deref() != Some(category_id);
        if category_changed || list_state.item_count() != total_items {
            self.current_viewed_category = Some(category_id.to_string());
            list_state.reset(total_items);
            let _measurement_task = list_state.clone().measure_all();
        }

        let view = cx.entity().clone();
        let state = self.state.clone();
        let cat_for_list = category.clone();
        let builtin_for_list = builtin.clone();
        let strategies_for_list = strategies.clone();
        let scroll_to_active_state = list_state.clone();

        let list_element = list(list_state.clone(), move |ix, _window, cx| {
            if ix == 0 {
                let add_id = cat_for_list.id.clone();
                let edit_category_id = cat_for_list.id.clone();
                let clear_category_id = cat_for_list.id.clone();
                let delete_category_id = cat_for_list.id.clone();
                let restore_id = cat_for_list.id.clone();
                let category_has_active = cat_for_list.strategies.iter().any(|s| s.active);

                let header = div()
                    .w_full()
                    .px_6()
                    .pt(PAGE_TOP_PADDING)
                    .pb(PAGE_HEADER_GAP)
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
                                    "back-to-strategies",
                                    "icons/arrow-left.svg",
                                    cx,
                                )
                                .ghost()
                                .small()
                                .on_click({
                                    let view = view.clone();
                                    move |_, _, cx| {
                                        view.update(cx, |this, cx| {
                                            this.navigate(Route::Strategies, cx);
                                        });
                                    }
                                }),
                            )
                            .child(
                                div()
                                    .min_w_0()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_1()
                                            .child(
                                                div()
                                                    .text_2xl()
                                                    .line_height(px(32.))
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .child(cat_for_list.name.clone()),
                                            )
                                            .child(source_kind_badge(
                                                SharedString::from(format!(
                                                    "header-badge-cat-{}",
                                                    cat_for_list.id
                                                )),
                                                cat_for_list.system,
                                                px(12.),
                                            ))
                                            .when_some(
                                                active_strategy.clone(),
                                                |title, (active_index, active)| {
                                                    let tooltip_text = t!(
                                                        "strategies.active_tooltip",
                                                        name = active.as_str()
                                                    );
                                                    let active_elem_id =
                                                        SharedString::from(format!(
                                                            "header-active-{}",
                                                            cat_for_list.id
                                                        ));
                                                    let scroll_state =
                                                        scroll_to_active_state.clone();
                                                    title.child(cursor_tooltip::attach(
                                                        div()
                                                            .id(active_elem_id.clone())
                                                            .flex()
                                                            .items_center()
                                                            .cursor_pointer()
                                                            .on_click(move |_, window, _| {
                                                                scroll_state.scroll_to_reveal_item(
                                                                    active_index,
                                                                );
                                                                window.refresh();
                                                            })
                                                            .child(pulsing_label(
                                                                SharedString::from(format!(
                                                                    "detail-active-{}",
                                                                    cat_for_list.id
                                                                )),
                                                                active,
                                                                success(),
                                                            )),
                                                        ElementId::from(active_elem_id),
                                                        tooltip_text,
                                                    ))
                                                },
                                            )
                                            .when(!category_has_active, |title| {
                                                let tooltip_text =
                                                    t!("strategies.inactive_tooltip");
                                                let inactive_elem_id = SharedString::from(format!(
                                                    "header-inactive-{}",
                                                    cat_for_list.id
                                                ));
                                                title.child(cursor_tooltip::attach(
                                                    div()
                                                        .id(inactive_elem_id.clone())
                                                        .flex()
                                                        .items_center()
                                                        .child(pulsing_dot(
                                                            SharedString::from(format!(
                                                                "detail-inactive-{}",
                                                                cat_for_list.id
                                                            )),
                                                            danger(),
                                                        )),
                                                    ElementId::from(inactive_elem_id),
                                                    tooltip_text,
                                                ))
                                            })
                                            .when(modified, |title| {
                                                let restore_id = restore_id.clone();
                                                let state = state.clone();
                                                let restore_elem_id = SharedString::from(format!(
                                                    "restore-category-inline-{}",
                                                    cat_for_list.id
                                                ));
                                                title.child(restore_badge(
                                                    restore_elem_id,
                                                    Some(px(16.)),
                                                    move |_, _, cx| {
                                                        state.update(cx, |state, cx| {
                                                            state.restore_category(&restore_id, cx)
                                                        })
                                                    },
                                                ))
                                            }),
                                    )
                                    .child(
                                        div()
                                            .mt_1()
                                            .text_sm()
                                            .line_height(px(20.))
                                            .text_color(muted_foreground())
                                            .child(strategies_count(strategies_for_list.len())),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(icon_button(
                                "add-strategy",
                                "icons/plus.svg",
                                IconButtonVariant::Primary,
                                {
                                    let view = view.clone();
                                    move |_, _, cx| {
                                        let cat = add_id.clone();
                                        view.update(cx, |this, cx| {
                                            this.open_strategy(cat, None, cx);
                                        });
                                    }
                                },
                                cx,
                            ))
                            .child(icon_button(
                                "rename-category",
                                "icons/pencil.svg",
                                IconButtonVariant::Outline,
                                {
                                    let view = view.clone();
                                    move |_, _, cx| {
                                        let cat = edit_category_id.clone();
                                        view.update(cx, |this, cx| {
                                            this.open_category(Some(cat), cx);
                                        });
                                    }
                                },
                                cx,
                            ))
                            .when(category_has_active, |actions| {
                                let state = state.clone();
                                actions.child(icon_button(
                                    "clear-category",
                                    "icons/brush-cleaning.svg",
                                    IconButtonVariant::Warning,
                                    move |_, _, cx| {
                                        let cat = clear_category_id.clone();
                                        state.update(cx, |state, cx| {
                                            state.clear_category(&cat, cx);
                                        });
                                    },
                                    cx,
                                ))
                            })
                            .when(cat_for_list.system && modified, |actions| {
                                let state = state.clone();
                                actions.child(icon_button(
                                    "restore-category",
                                    "icons/rotate-ccw.svg",
                                    IconButtonVariant::Warning,
                                    move |_, _, cx| {
                                        let cat = restore_id.clone();
                                        state.update(cx, |state, cx| {
                                            state.restore_category(&cat, cx);
                                        });
                                    },
                                    cx,
                                ))
                            })
                            .child(icon_button(
                                "delete-category",
                                "icons/trash-2.svg",
                                IconButtonVariant::Destructive,
                                {
                                    let state = state.clone();
                                    move |_, _, cx| {
                                        let cat_name = delete_category_id.clone();
                                        state.update(cx, |state, cx| {
                                            let name = state
                                                .config
                                                .categories
                                                .iter()
                                                .find(|c| c.id == cat_name)
                                                .map(|c| c.name.clone())
                                                .unwrap_or_else(|| cat_name.clone());
                                            state.set_confirm(
                                                Some(ConfirmTarget::DeleteCategory {
                                                    id: cat_name.clone(),
                                                    name,
                                                }),
                                                cx,
                                            );
                                        });
                                    }
                                },
                                cx,
                            )),
                    );
                return header.into_any_element();
            }

            if strategies_for_list.is_empty() {
                return div()
                    .w_full()
                    .px_6()
                    .pb_4()
                    .child(
                        div()
                            .text_sm()
                            .text_color(muted_foreground())
                            .child(t!("strategies.empty")),
                    )
                    .into_any_element();
            }

            let strategy_ix = ix - 1;
            let strategy = &strategies_for_list[strategy_ix];
            let edit_category = cat_for_list.id.clone();
            let edit_id = strategy.id.clone();
            let builtin_strategy = builtin_for_list
                .as_ref()
                .and_then(|b| b.strategies.iter().find(|s| s.id == strategy.id))
                .cloned();

            let card = strategy_card(
                cat_for_list.id.clone(),
                strategy.clone(),
                builtin_strategy,
                state.clone(),
                {
                    let view = view.clone();
                    move |_, _, cx| {
                        let edit_cat = edit_category.clone();
                        let edit_strat = edit_id.clone();
                        view.update(cx, |this, cx| {
                            this.open_strategy(edit_cat, Some(edit_strat), cx);
                        });
                    }
                },
                cx,
            );

            div().w_full().px_6().pb_4().child(card).into_any_element()
        });

        div()
            .size_full()
            .relative()
            .overflow_hidden()
            .child(
                SmoothListScroll::new(
                    format!("category-page-{}", page_category_id),
                    list_state.clone(),
                    list_element.size_full(),
                )
                .scroll_to_top(true),
            )
            .child(PageScrollbar::new(
                SharedString::from(format!("scrollbar-category-{page_category_id}")),
                list_state,
            ))
            .into_any_element()
    }
}
