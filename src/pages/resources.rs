use super::*;
use crate::ui::components::button::Button;

impl AppView {
    pub(crate) fn filters_page(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let (filters, builtin_filters) = {
            let app_state = self.state.read(cx);
            (
                app_state.config.filters.clone(),
                app_state.builtin.filters.clone(),
            )
        };
        let count = filters.len() + 1;
        let scroll_handle = self.filters_scroll_handle.clone();
        let state = self.state.clone();
        let open_state = self.state.clone();
        let view = cx.entity().clone();

        let list = uniform_list(
            "filters-virtual-list",
            count,
            move |range, _window, cx| {
                range
                    .map(|row| {
                        if row == 0 {
                            let view_add = view.clone();
                            let open_state_header = open_state.clone();
                            let actions = div()
                                .flex()
                                .gap_1()
                                .child(
                                    Button::new(
                                        "add-filter",
                                        t!("filters.btn_add"),
                                        cx,
                                    )
                                    .primary()
                                    .icon_suffix("icons/plus.svg")
                                    .on_click(move |_, _, cx| {
                                        view_add.update(cx, |this, cx| this.open_filter(None, cx));
                                    }),
                                )
                                .child(icon_button(
                                    "open-filters",
                                    "icons/folder-open.svg",
                                    IconButtonVariant::Outline,
                                    move |_, _, cx| {
                                        open_state_header.update(cx, |state, cx| state.open_filters_directory(cx))
                                    },
                                    cx,
                                ));
                            return virtual_header_row(
                                t!("filters.title"),
                                Some(t!("filters.desc")),
                                actions,
                            );
                        }

                        let ix = row - 1;
                        let filter = &filters[ix];
                        let toggle_id = filter.id.clone();
                        let edit_id = filter.id.clone();
                        let delete_id = filter.id.clone();
                        let restore_id = filter.id.clone();
                        let toggle_state = state.clone();
                        let delete_state = state.clone();
                        let restore_state = state.clone();
                        let view_edit = view.clone();
                        let system = filter.system;
                        let active = filter.active;
                        let switch = switch(
                            SharedString::from(format!("filter-switch-{toggle_id}")),
                            active,
                            cx,
                        )
                        .on_toggle(move |_, _, cx| {
                            toggle_state.update(cx, |state, cx| {
                                state.toggle_filter(&toggle_id, cx)
                            })
                        });
                        let modified = builtin_filters
                            .iter()
                            .find(|builtin| builtin.id == filter.id)
                            .is_some_and(|builtin| filter_modified(filter, builtin));
                        let title = filter.name.clone();
                        let filename = filter.filename.clone();

                        virtual_list_row(
                                virtual_list_card()
                                    .child(card_icon("icons/funnel.svg", muted_foreground()))
                                    .child(
                                        div()
                                            .min_w_0()
                                            .flex()
                                            .flex_1()
                                            .items_center()
                                            .justify_between()
                                            .gap_3()
                                            .child(
                                                div()
                                                    .min_w_0()
                                                    .flex_1()
                                                    .flex()
                                                    .flex_col()
                                                    .gap_1()
                                                    .child(
                                                        div()
                                                            .min_w_0()
                                                            .flex()
                                                            .items_center()
                                                            .gap_1()
                                                            .text_sm()
                                                            .line_height(px(20.))
                                                            .child(div().min_w_0().truncate().child(title.clone()))
                                                            .child(source_kind_badge(
                                                                SharedString::from(format!(
                                                                    "badge-filter-{}",
                                                                    filter.id
                                                                )),
                                                                system,
                                                                px(12.),
                                                            ))
                                                            .when(modified, |title| {
                                                                let restore_id_str = SharedString::from(format!("restore-filter-inline-{restore_id}"));
                                                                title.child(restore_badge(
                                                                    restore_id_str,
                                                                    None,
                                                                    move |_, _, cx| {
                                                                        restore_state.update(cx, |state, cx| {
                                                                            state.restore_filter(&restore_id, cx)
                                                                        });
                                                                    },
                                                                ))
                                                            }),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_xs()
                                                            .line_height(px(16.))
                                                            .font_family("IBM Plex Mono")
                                                            .text_color(muted_foreground())
                                                            .truncate()
                                                            .child(filename),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .gap_1()
                                                    .child(switch)
                                                    .child(icon_button(
                                                        SharedString::from(format!("edit-filter-{edit_id}")),
                                                        "icons/pencil.svg",
                                                        IconButtonVariant::Outline,
                                                        move |_, _, cx| {
                                                            view_edit.update(cx, |this, cx| {
                                                                this.open_filter(Some(edit_id.clone()), cx);
                                                            });
                                                        },
                                                        cx,
                                                    ))
                                                    .child(icon_button(
                                                        SharedString::from(format!("delete-filter-{delete_id}")),
                                                        "icons/trash-2.svg",
                                                        IconButtonVariant::Destructive,
                                                        {
                                                            let name = title.clone();
                                                            let id = delete_id.clone();
                                                            move |_, _, cx| {
                                                                delete_state.update(cx, |state, cx| {
                                                                    state.set_confirm(
                                                                        Some(crate::ui::components::confirm_dialog::ConfirmTarget::DeleteFilter {
                                                                            id: id.clone(),
                                                                            name: name.clone(),
                                                                        }),
                                                                        cx,
                                                                    )
                                                                })
                                                            }
                                                        },
                                                        cx,
                                                    )),
                                            ),
                                    ),
                            )
                            .into_any_element()
                    })
                    .collect()
            },
        )
        .track_scroll(&scroll_handle);

        virtual_page_container("filters", scroll_handle, list, None)
    }

    pub(crate) fn placeholders_page(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let (placeholders, builtin_placeholders, resources_root) = {
            let app_state = self.state.read(cx);
            (
                app_state.config.placeholders.clone(),
                app_state.builtin.placeholders.clone(),
                app_state.config.binaries_path.clone(),
            )
        };
        let count = placeholders.len() + 1;
        let scroll_handle = self.placeholders_scroll_handle.clone();
        let state = self.state.clone();
        let open_state = self.state.clone();
        let view = cx.entity().clone();

        let list = uniform_list(
            "placeholders-virtual-list",
            count,
            move |range, _window, cx| {
                range
                    .map(|row| {
                        if row == 0 {
                            let view_add = view.clone();
                            let open_state_header = open_state.clone();
                            let actions = div()
                                .flex()
                                .gap_1()
                                .child(
                                    Button::new(
                                        "add-placeholder",
                                        t!("placeholders.btn_add"),
                                        cx,
                                    )
                                    .primary()
                                    .icon_suffix("icons/plus.svg")
                                    .on_click(move |_, _, cx| {
                                        view_add.update(cx, |this, cx| this.open_placeholder(None, cx));
                                    }),
                                )
                                .child(icon_button(
                                    "open-placeholders",
                                    "icons/folder-open.svg",
                                    IconButtonVariant::Outline,
                                    move |_, _, cx| {
                                        open_state_header.update(cx, |state, cx| {
                                            state.open_placeholders_directory(cx)
                                        })
                                    },
                                    cx,
                                ));
                            return virtual_header_row(
                                t!("placeholders.title"),
                                Some(t!("placeholders.desc")),
                                actions,
                            );
                        }

                        let index = row - 1;
                        let item = &placeholders[index];
                        let state = state.clone();
                        let restore_state = state.clone();
                        let view_edit = view.clone();
                        let restore_name = item.name.clone();
                        let system = item.system;
                        let modified = builtin_placeholders
                            .iter()
                            .find(|builtin| {
                                builtin.name == item.name
                                    || item.system_base_name.as_deref() == Some(builtin.name.as_str())
                            })
                            .is_some_and(|builtin| placeholder_modified(item, builtin));
                        let title = format!("{{{{{}}}}}", item.name);
                        let path = display_placeholder_path(&item.path, &resources_root);

                        virtual_list_row(
                                virtual_list_card()
                                    .child(card_icon("icons/file-code.svg", muted_foreground()))
                                    .child(
                                        div()
                                            .min_w_0()
                                            .flex()
                                            .flex_1()
                                            .items_center()
                                            .justify_between()
                                            .gap_3()
                                            .child(
                                                div()
                                                    .min_w_0()
                                                    .flex_1()
                                                    .flex()
                                                    .flex_col()
                                                    .gap_1()
                                                    .child(
                                                        div()
                                                            .min_w_0()
                                                            .flex()
                                                            .items_center()
                                                            .gap_1()
                                                            .text_sm()
                                                            .line_height(px(20.))
                                                            .child(div().min_w_0().truncate().child(title))
                                                            .child(source_kind_badge(
                                                                SharedString::from(format!(
                                                                    "badge-placeholder-{index}"
                                                                )),
                                                                system,
                                                                px(12.),
                                                            ))
                                                            .when(modified, |title| {
                                                                let restore_id_str = SharedString::from(format!("restore-placeholder-inline-{index}"));
                                                                title.child(restore_badge(
                                                                    restore_id_str,
                                                                    None,
                                                                    move |_, _, cx| {
                                                                        restore_state.update(cx, |state, cx| {
                                                                            state.restore_placeholder(
                                                                                &restore_name,
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
                                                            .font_family("IBM Plex Mono")
                                                            .text_color(muted_foreground())
                                                            .truncate()
                                                            .child(path),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .gap_1()
                                                    .child(icon_button(
                                                        SharedString::from(format!("edit-placeholder-{index}")),
                                                        "icons/pencil.svg",
                                                        IconButtonVariant::Outline,
                                                        move |_, _, cx| {
                                                            view_edit.update(cx, |this, cx| {
                                                                this.open_placeholder(Some(index), cx);
                                                            });
                                                        },
                                                        cx,
                                                    ))
                                                    .child(icon_button(
                                                        SharedString::from(format!("delete-placeholder-{index}")),
                                                        "icons/trash-2.svg",
                                                        IconButtonVariant::Destructive,
                                                        {
                                                            let name = item.name.clone();
                                                            move |_, _, cx| {
                                                                state.update(cx, |state, cx| {
                                                                    state.set_confirm(
                                                                        Some(crate::ui::components::confirm_dialog::ConfirmTarget::DeletePlaceholder {
                                                                            index,
                                                                            name: name.clone(),
                                                                        }),
                                                                        cx,
                                                                    )
                                                                })
                                                            }
                                                        },
                                                        cx,
                                                    )),
                                            ),
                                    ),
                            )
                            .into_any_element()
                    })
                    .collect()
            },
        )
        .track_scroll(&scroll_handle);

        virtual_page_container("placeholders", scroll_handle, list, None)
    }
}

fn display_placeholder_path(path: &str, resources_root: &str) -> String {
    path.strip_prefix("@resources").map_or_else(
        || path.to_owned(),
        |relative| {
            std::path::Path::new(resources_root)
                .join(relative.trim_start_matches(['/', '\\']))
                .to_string_lossy()
                .replace(['/', '\\'], std::path::MAIN_SEPARATOR_STR)
        },
    )
}

#[cfg(test)]
mod tests {
    use super::display_placeholder_path;

    #[test]
    fn placeholder_resource_alias_is_expanded_for_display() {
        let path = display_placeholder_path("@resources/fake/test.bin", "D:\\ThirdParty");
        assert!(path.ends_with("ThirdParty\\fake\\test.bin"));
    }
}
