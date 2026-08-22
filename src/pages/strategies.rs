use std::cell::Cell;
use std::rc::Rc;

use super::category_drag::{self, CategoryDrag, ProjectedCategory};
use super::*;
use crate::ui::components::button::Button;
use crate::ui::components::cursor_tooltip;
use crate::ui::foundation::hover_motion;

impl AppView {
    pub(crate) fn strategies_page(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let (categories, builtin_categories) = {
            let app_state = self.state.read(cx);
            (
                app_state.config.categories.clone(),
                app_state.builtin.categories.clone(),
            )
        };
        let active_drag = category_drag::active(cx);
        let layout_transition = category_drag::layout_transition(cx);
        let drag_preview = category_drag::preview_layout(cx);
        let projected = category_drag::projected_categories(&categories, active_drag);
        let count = projected.len() + 1;
        let list_bounds = Rc::new(Cell::new(None));
        let is_dragging = active_drag.is_some();
        let scroll_handle = self.categories_scroll_handle.clone();
        let state = self.state.clone();
        let view = cx.entity().clone();

        let list = uniform_list(
            "strategies-virtual-list",
            count,
            move |range, _window, cx| {
                range
                    .map(|row| {
                        if row == 0 {
                            let view_add = view.clone();
                            let actions =
                                Button::new("add-category", t!("strategies.btn_add_category"), cx)
                                    .primary()
                                    .icon_suffix("icons/plus.svg")
                                    .on_click(move |_, _, cx| {
                                        view_add
                                            .update(cx, |this, cx| this.open_category(None, cx));
                                    });
                            return virtual_header_row(
                                t!("strategies.title"),
                                Some(t!("strategies.desc")),
                                actions,
                            );
                        }

                        let ix = row - 1;
                        let row = &projected[ix];
                        let row_id = row.id();
                        let (index, category) = match row {
                            ProjectedCategory::Item { index, category } => {
                                (*index, category.clone())
                            }
                            ProjectedCategory::Placeholder(category) => {
                                let element =
                                    category_drag::placeholder(category.clone(), state.clone());
                                return virtual_list_row(category_drag::animate_row(
                                    element,
                                    row_id,
                                    layout_transition,
                                ))
                                .into_any_element();
                            }
                        };
                        let open_id = category.id.clone();
                        let edit_id = category.id.clone();
                        let delete_id = category.id.clone();
                        let clear_id = category.id.clone();
                        let row_state = state.clone();
                        let clear = state.clone();
                        let restore = state.clone();
                        let view_open = view.clone();
                        let view_edit = view.clone();
                        let system = category.system;
                        let modified = builtin_categories
                            .iter()
                            .find(|builtin| builtin.id == category.id)
                            .is_some_and(|builtin| category_modified(&category, builtin));
                        let restore_id = category.id.clone();
                        let active_strategy = category
                            .strategies
                            .iter()
                            .find(|item| item.active)
                            .map(|item| item.name.clone());
                        let has_active_strategy = active_strategy.is_some();
                        let status_id = category.id.clone();
                        let strategy_count = category.strategies.len();
                        let drag = CategoryDrag {
                            id: category.id.clone(),
                            from_index: index,
                            category: category.clone(),
                            source_bounds: Rc::new(Cell::new(None)),
                            list_bounds: Rc::clone(&list_bounds),
                            grab_offset: Rc::new(Cell::new(Point::default())),
                        };
                        let measured_source_bounds = Rc::clone(&drag.source_bounds);
                        let drop_state = state.clone();
                        let hover_key = SharedString::from(format!("cat-hover-{}", category.id));
                        let hover_key_clone = hover_key.clone();
                        let hover_progress = hover_motion::progress(&hover_key, cx);

                        let element = virtual_list_card()
                            .relative()
                            .group("category-row")
                            .on_prepaint(move |bounds, _, _| {
                                measured_source_bounds.set(Some(bounds))
                            })
                            .child(
                                card_icon("icons/grip-vertical.svg", muted_foreground())
                                    .id(SharedString::from(format!("drag-{}", category.id)))
                                    .relative()
                                    .cursor_grab()
                                    .active(|style| style.cursor_grabbing())
                                    .on_drag(drag, |drag, _, window, cx| {
                                        if let Some(source_bounds) = drag.source_bounds.get() {
                                            drag.grab_offset.set(
                                                window.mouse_position() - source_bounds.origin,
                                            );
                                        }
                                        category_drag::begin(drag, window.mouse_position(), cx);
                                        cx.refresh_windows();
                                        cx.new(|_| category_drag::InvisibleDragPreview)
                                    }),
                            )
                            .child(
                                div()
                                    .id(SharedString::from(format!("open-{open_id}")))
                                    .flex_1()
                                    .relative()
                                    .self_stretch()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .cursor_pointer()
                                    .on_hover(move |hovered, window, cx| {
                                        hover_motion::set_hovered(
                                            hover_key_clone.clone(),
                                            *hovered,
                                            window,
                                            cx,
                                        );
                                    })
                                    .on_click(move |_, _, cx| {
                                        view_open.update(cx, |this, cx| {
                                            this.navigate(Route::Category(open_id.clone()), cx);
                                        });
                                    })
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
                                                    .child(category.name)
                                                    .child(source_kind_badge(
                                                        SharedString::from(format!(
                                                            "badge-cat-{}",
                                                            category.id
                                                        )),
                                                        system,
                                                        px(12.),
                                                    ))
                                                    .when_some(active_strategy, |title, active| {
                                                        let tooltip_text = t!(
                                                            "strategies.active_tooltip",
                                                            name = active.as_str()
                                                        );
                                                        let active_elem_id = SharedString::from(
                                                            format!("category-active-{status_id}"),
                                                        );
                                                        title.child(cursor_tooltip::attach(
                                                            div()
                                                                .id(active_elem_id.clone())
                                                                .flex()
                                                                .items_center()
                                                                .child(pulsing_label(
                                                                    SharedString::from(format!(
                                                                        "anim-active-{status_id}"
                                                                    )),
                                                                    active,
                                                                    success(),
                                                                )),
                                                            ElementId::from(active_elem_id),
                                                            tooltip_text,
                                                        ))
                                                    })
                                                    .when(!has_active_strategy, |title| {
                                                        let tooltip_text =
                                                            t!("strategies.inactive_tooltip");
                                                        let inactive_elem_id =
                                                            SharedString::from(format!(
                                                                "category-inactive-{status_id}"
                                                            ));
                                                        title.child(cursor_tooltip::attach(
                                                            div()
                                                                .id(inactive_elem_id.clone())
                                                                .flex()
                                                                .items_center()
                                                                .child(pulsing_dot(
                                                                    SharedString::from(format!(
                                                                        "anim-inactive-{status_id}"
                                                                    )),
                                                                    danger(),
                                                                )),
                                                            ElementId::from(inactive_elem_id),
                                                            tooltip_text,
                                                        ))
                                                    })
                                                    .when(modified, |title| {
                                                        let restore_id_str = SharedString::from(
                                                            format!("restore-inline-{restore_id}"),
                                                        );
                                                        title.child(restore_badge(
                                                            restore_id_str,
                                                            None,
                                                            move |_, _, cx| {
                                                                restore.update(cx, |state, cx| {
                                                                    state.restore_category(
                                                                        &restore_id,
                                                                        cx,
                                                                    )
                                                                });
                                                            },
                                                        ))
                                                    }),
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .line_height(px(16.))
                                                    .text_color(muted_foreground())
                                                    .child(strategies_count(strategy_count)),
                                            ),
                                    )
                                    .child(
                                        svg()
                                            .path("icons/chevron-right.svg")
                                            .size_4()
                                            .relative()
                                            .left(px(hover_progress * 5.0))
                                            .text_color(mix_color(
                                                muted_foreground(),
                                                foreground(),
                                                hover_progress,
                                            )),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .relative()
                                    .child(icon_button(
                                        SharedString::from(format!("edit-{edit_id}")),
                                        "icons/pencil.svg",
                                        IconButtonVariant::Outline,
                                        move |_, _, cx| {
                                            view_edit.update(cx, |this, cx| {
                                                this.open_category(Some(edit_id.clone()), cx);
                                            });
                                        },
                                        cx,
                                    ))
                                    .when(has_active_strategy, |actions| {
                                        actions.child(icon_button(
                                            SharedString::from(format!("clear-{clear_id}")),
                                            "icons/brush-cleaning.svg",
                                            IconButtonVariant::Warning,
                                            move |_, _, cx| {
                                                clear.update(cx, |state, cx| {
                                                    state.clear_category(&clear_id, cx)
                                                })
                                            },
                                            cx,
                                        ))
                                    })
                                    .child(icon_button(
                                        SharedString::from(format!("delete-{delete_id}")),
                                        "icons/trash-2.svg",
                                        IconButtonVariant::Destructive,
                                        move |_, _, cx| {
                                            row_state.update(cx, |state, cx| {
                                                state.delete_category(&delete_id, cx)
                                            })
                                        },
                                        cx,
                                    )),
                            )
                            .when(is_dragging, |row| {
                                row.children(category_drag::drop_zones(index, drop_state))
                            })
                            .into_any_element();

                        let animated =
                            category_drag::animate_row(element, row_id, layout_transition);
                        virtual_list_row(animated).into_any_element()
                    })
                    .collect()
            },
        )
        .track_scroll(&scroll_handle);

        let overlay = drag_preview.map(|preview| {
            div()
                .absolute()
                .inset_0()
                .child(category_drag::render_preview(preview, cx))
                .into_any_element()
        });

        virtual_page_container("strategies", scroll_handle, list, overlay)
    }
}
