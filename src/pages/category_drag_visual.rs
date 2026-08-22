use gpui::prelude::*;
use gpui::*;

use super::category_drag::DragPreviewLayout;
use super::{pulsing_dot, pulsing_label, source_kind_icon, strategies_count};
use crate::domain::Category;
use crate::ui::components::card::{card_icon, virtual_list_card};
use crate::ui::foundation::colors::{destructive as danger, foreground, muted_foreground, success};

use crate::ui::components::button::{IconButton, IconButtonVariant};

pub(super) fn render_preview(preview: DragPreviewLayout, cx: &App) -> AnyElement {
    div()
        .absolute()
        .left_0()
        .top(preview.top)
        .w_full()
        .h(preview.height)
        .px_6()
        .opacity(0.98)
        .shadow_xl()
        .cursor_grabbing()
        .child(category_preview_visual(&preview.category, cx))
        .into_any_element()
}

fn category_preview_visual(category: &Category, cx: &App) -> Div {
    let system = category.system;
    let active_strategy = category
        .strategies
        .iter()
        .find(|item| item.active)
        .map(|item| item.name.clone());
    let has_active_strategy = active_strategy.is_some();

    virtual_list_card()
        .size_full()
        .child(card_icon("icons/grip-vertical.svg", muted_foreground()).cursor_grabbing())
        .child(
            div()
                .flex_1()
                .relative()
                .self_stretch()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_1()
                                .truncate()
                                .text_sm()
                                .line_height(px(20.))
                                .font_weight(FontWeight::NORMAL)
                                .text_color(foreground())
                                .child(category.name.clone())
                                .child(source_kind_icon(system, px(12.)))
                                .when_some(active_strategy, |title, active| {
                                    title.child(pulsing_label(
                                        SharedString::from(format!(
                                            "preview-active-{}",
                                            category.id
                                        )),
                                        active,
                                        success(),
                                    ))
                                })
                                .when(!has_active_strategy, |title| {
                                    title.child(pulsing_dot(
                                        SharedString::from(format!(
                                            "preview-inactive-{}",
                                            category.id
                                        )),
                                        danger(),
                                    ))
                                }),
                        )
                        .child(
                            div()
                                .text_xs()
                                .line_height(px(16.))
                                .text_color(muted_foreground())
                                .child(strategies_count(category.strategies.len())),
                        ),
                )
                .child(
                    svg()
                        .path("icons/chevron-right.svg")
                        .size_4()
                        .text_color(muted_foreground()),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_1()
                .relative()
                .child(
                    IconButton::new("preview-edit", "icons/pencil.svg", cx)
                        .variant(IconButtonVariant::Outline)
                        .into_element(),
                )
                .when(has_active_strategy, |actions| {
                    actions.child(
                        IconButton::new("preview-clear", "icons/brush-cleaning.svg", cx)
                            .variant(IconButtonVariant::Warning)
                            .into_element(),
                    )
                })
                .child(
                    IconButton::new("preview-delete", "icons/trash-2.svg", cx)
                        .variant(IconButtonVariant::Destructive)
                        .into_element(),
                ),
        )
}
