use gpui::prelude::*;
use gpui::*;

use crate::app::{AppView, EditorTarget, Route};
use crate::domain::{Category, Filter, ListMode, Placeholder};
use crate::services::dns::PRESETS;
use crate::ui::components::backdrop_blur::backdrop_blur;
use crate::ui::components::button::{Button, IconButtonVariant, icon_button};
use crate::ui::components::card::{
    card_icon, interactive_module_card, module_card, module_header, virtual_list_card,
};
use crate::ui::components::cursor_tooltip;
use crate::ui::components::disclosure::{DisclosureChevron, disclosure_progress};
use crate::ui::components::smooth_scroll::{
    PageScrollbar, SmoothUniformListScroll, SmoothVerticalScroll,
};
use crate::ui::components::text_input::TextInput;
use crate::ui::components::toggle_switch::{animate_toggle, switch};
use crate::ui::foundation::colors::{
    self, accent_foreground, background, border, card as card_color, destructive as danger,
    foreground, muted_foreground, success, warning, yellow as accent,
};
use crate::ui::foundation::element_ext::ElementPrepaintExt as _;
use crate::ui::foundation::motion::mix_color;

mod about;
mod category_drag;
mod category_drag_visual;
mod collections;
mod configuration;
mod core;
mod editor;
mod logs;
mod module_detail;
mod probe;
mod probe_results;
mod resources;
mod settings;
mod strategies;
mod strategy_card;

pub(crate) fn init(cx: &mut App) {
    category_drag::init(cx);
}

pub(crate) fn update_category_drag_mouse(position: Point<Pixels>, cx: &mut App) -> bool {
    category_drag::update_mouse_position(position, cx)
}

pub(crate) const VIRTUAL_ROW_HEIGHT: Pixels = px(88.0);
const PAGE_TOP_PADDING: Pixels = px(20.0);
const PAGE_HEADER_GAP: Pixels = px(12.0);

fn page(title: impl Into<SharedString>, content: impl IntoElement) -> AnyElement {
    page_with_actions(title, div(), content)
}

fn virtual_page_container(
    id: impl Into<SharedString>,
    handle: UniformListScrollHandle,
    list: UniformList,
    overlay: Option<AnyElement>,
) -> AnyElement {
    let id = id.into();
    div()
        .size_full()
        .relative()
        .overflow_hidden()
        .text_color(foreground())
        .on_prepaint(|bounds, _, cx| {
            category_drag::set_list_bounds(bounds, cx);
        })
        .child(
            SmoothUniformListScroll::new(
                SharedString::from(format!("smooth-virtual-{id}")),
                handle.clone(),
                list.size_full(),
            )
            .scroll_to_top(true),
        )
        .child(PageScrollbar::new(
            SharedString::from(format!("scrollbar-virtual-{id}")),
            handle,
        ))
        .children(overlay)
        .into_any_element()
}

fn virtual_list_row(child: impl IntoElement) -> Div {
    div()
        .w_full()
        .h(VIRTUAL_ROW_HEIGHT)
        .px_6()
        .pb_4()
        .child(child)
}

fn virtual_header_row(
    title: impl Into<SharedString>,
    desc: Option<std::borrow::Cow<'static, str>>,
    actions: impl IntoElement,
) -> AnyElement {
    let title = title.into();
    div()
        .w_full()
        .h(VIRTUAL_ROW_HEIGHT)
        .px_6()
        .pt(PAGE_TOP_PADDING)
        .pb(PAGE_HEADER_GAP)
        .child(
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .justify_center()
                        .child(
                            div()
                                .text_2xl()
                                .line_height(px(32.))
                                .font_weight(FontWeight::MEDIUM)
                                .child(title),
                        )
                        .when_some(desc, |header, description| {
                            header.child(
                                div()
                                    .mt_1()
                                    .text_sm()
                                    .line_height(px(20.))
                                    .text_color(muted_foreground())
                                    .child(description),
                            )
                        }),
                )
                .child(actions),
        )
        .into_any_element()
}

fn page_with_actions(
    title: impl Into<SharedString>,
    actions: impl IntoElement,
    content: impl IntoElement,
) -> AnyElement {
    let title = title.into();
    let desc = page_description(&title);
    let enable_scroll_to_top = title.as_ref() == t!("strategies.title")
        || title.as_ref() == t!("filters.title")
        || title.as_ref() == t!("placeholders.title");
    div()
        .size_full()
        .child(
            SmoothVerticalScroll::new(
                SharedString::from(format!("page-{title}")),
                div()
                    .min_h_full()
                    .px_6()
                    .pt(PAGE_TOP_PADDING)
                    .pb_6()
                    .flex()
                    .flex_col()
                    .gap(PAGE_HEADER_GAP)
                    .text_color(foreground())
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .text_2xl()
                                            .line_height(px(32.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .child(title.clone()),
                                    )
                                    .when_some(desc, |header, description| {
                                        header.child(
                                            div()
                                                .mt_1()
                                                .text_sm()
                                                .line_height(px(20.))
                                                .text_color(muted_foreground())
                                                .child(description),
                                        )
                                    }),
                            )
                            .child(actions),
                    )
                    .child(content),
            )
            .scroll_to_top(enable_scroll_to_top),
        )
        .into_any_element()
}

fn page_description(title: &str) -> Option<std::borrow::Cow<'static, str>> {
    if title == t!("strategies.title") {
        Some(t!("strategies.desc"))
    } else if title == t!("filters.title") {
        Some(t!("filters.desc"))
    } else if title == t!("placeholders.title") {
        Some(t!("placeholders.desc"))
    } else if title == t!("modules.title") {
        Some(t!("modules.desc"))
    } else if title == t!("logs.title") {
        Some(t!("logs.desc"))
    } else if title == t!("settings.title") {
        Some(t!("settings.desc"))
    } else if title == t!("about.title") {
        Some(t!("about.desc"))
    } else if title == t!("modules.dns_title") {
        Some(t!("modules.dns_desc"))
    } else if title == t!("modules.tg_proxy_title") {
        Some(t!("modules.tg_proxy_desc"))
    } else {
        None
    }
}

fn strategies_count(count: usize) -> String {
    let last_two = count % 100;
    if (11..=14).contains(&last_two) {
        return format!("{count} стратегий");
    }
    match count % 10 {
        1 => format!("{count} стратегия"),
        2..=4 => format!("{count} стратегии"),
        _ => format!("{count} стратегий"),
    }
}

fn pulsing_label(id: SharedString, label: String, color: Rgba) -> AnyElement {
    div()
        .text_xs()
        .line_height(px(16.))
        .text_color(color)
        .child(label)
        .with_animation(
            id,
            Animation::new(crate::ui::foundation::motion::PULSE_MOTION).repeat(),
            |element, delta| {
                let wave = crate::ui::foundation::motion::stepped_pulse(delta, 30);
                element.opacity(1.0 - 0.30 * wave)
            },
        )
        .into_any_element()
}

fn pulsing_dot(id: SharedString, color: Rgba) -> AnyElement {
    div()
        .size(px(8.))
        .rounded_full()
        .bg(color)
        .with_animation(
            id,
            Animation::new(crate::ui::foundation::motion::PULSE_MOTION).repeat(),
            |element, delta| {
                let wave = crate::ui::foundation::motion::stepped_pulse(delta, 30);
                element.opacity(1.0 - 0.30 * wave)
            },
        )
        .into_any_element()
}

fn category_modified(category: &Category, builtin: &Category) -> bool {
    category.name != builtin.name
        || category.strategies.len() != builtin.strategies.len()
        || category
            .strategies
            .iter()
            .zip(&builtin.strategies)
            .any(|(current, base)| {
                current.id != base.id
                    || current.name != base.name
                    || current.content != base.content
            })
}

fn filter_modified(filter: &Filter, builtin: &Filter) -> bool {
    filter.name != builtin.name
        || filter.filename != builtin.filename
        || filter.content != builtin.content
}

fn placeholder_modified(placeholder: &Placeholder, builtin: &Placeholder) -> bool {
    placeholder.name != builtin.name || placeholder.path != builtin.path
}

fn source_kind_icon(system: bool, size: Pixels) -> Svg {
    svg()
        .path(if system {
            "icons/package.svg"
        } else {
            "icons/user-round-plus.svg"
        })
        .size(size)
        .text_color(if system {
            muted_foreground()
        } else {
            accent().opacity(0.8)
        })
}

fn source_kind_badge(id: SharedString, system: bool, size: Pixels) -> AnyElement {
    cursor_tooltip::attach(
        div()
            .id(id.clone())
            .flex()
            .items_center()
            .child(source_kind_icon(system, size)),
        ElementId::from(id),
        if system {
            t!("strategies.badge_system")
        } else {
            t!("strategies.badge_user")
        },
    )
    .into_any_element()
}

fn restore_badge(
    id: SharedString,
    size: Option<Pixels>,
    click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    cursor_tooltip::attach(
        div()
            .id(id.clone())
            .when_some(size, |marker, size| marker.size(size))
            .cursor_pointer()
            .on_click(move |event, window, cx| {
                cx.stop_propagation();
                click(event, window, cx);
            })
            .child(
                svg()
                    .path("icons/rotate-ccw.svg")
                    .size(px(12.))
                    .text_color(danger()),
            ),
        ElementId::from(id),
        t!("strategies.restore_tooltip"),
    )
    .into_any_element()
}
