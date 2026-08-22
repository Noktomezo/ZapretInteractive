use crate::ui::foundation::colors;
use crate::ui::foundation::hover_motion;
use crate::ui::foundation::motion::mix_color;
use gpui::prelude::*;
use gpui::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CardVariant {
    #[default]
    Default,
    Interactive,
    Outline,
    Muted,
    Success,
    Warning,
    Destructive,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CardRadius {
    None,
    Sm,
    #[default]
    Md,
    Lg,
    Xl,
}

impl CardRadius {
    pub fn to_pixels(self) -> Pixels {
        match self {
            Self::None => px(0.),
            Self::Sm => px(6.),
            Self::Md => px(8.),
            Self::Lg => px(12.),
            Self::Xl => px(16.),
        }
    }
}

/// Unified Card component representing a structured visual container.
pub struct Card {
    id: Option<ElementId>,
    variant: CardVariant,
    radius: CardRadius,
    children: Vec<AnyElement>,
    interactive: bool,
    hover_key: Option<SharedString>,
    hover_progress: f32,
    on_click: Option<ClickHandler>,
    min_h: Option<Pixels>,
}

type ClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

impl Default for Card {
    fn default() -> Self {
        Self::new()
    }
}

impl Card {
    pub fn new() -> Self {
        Self {
            id: None,
            variant: CardVariant::Default,
            radius: CardRadius::Md,
            children: Vec::new(),
            interactive: false,
            hover_key: None,
            hover_progress: 0.0,
            on_click: None,
            min_h: None,
        }
    }

    pub fn interactive(id: impl Into<ElementId>, cx: &App) -> Self {
        let element_id: ElementId = id.into();
        let hover_key: SharedString = SharedString::from(format!("card-hover-{element_id:?}"));
        let hover_progress = hover_motion::progress(&hover_key, cx);
        Self {
            id: Some(element_id),
            variant: CardVariant::Interactive,
            radius: CardRadius::Md,
            children: Vec::new(),
            interactive: true,
            hover_key: Some(hover_key),
            hover_progress,
            on_click: None,
            min_h: None,
        }
    }

    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn variant(mut self, variant: CardVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn radius(mut self, radius: CardRadius) -> Self {
        self.radius = radius;
        self
    }

    pub fn rounded_lg(mut self) -> Self {
        self.radius = CardRadius::Lg;
        self
    }

    pub fn rounded_xl(mut self) -> Self {
        self.radius = CardRadius::Xl;
        self
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }

    pub fn children(mut self, children: impl IntoIterator<Item = impl IntoElement>) -> Self {
        self.children
            .extend(children.into_iter().map(|c| c.into_any_element()));
        self
    }

    pub fn min_h(mut self, h: Pixels) -> Self {
        self.min_h = Some(h);
        self
    }

    pub fn on_click(
        mut self,
        click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        debug_assert!(
            self.interactive,
            "Card::on_click requires Card::interactive(id, cx)"
        );
        self.on_click = Some(Box::new(click));
        self
    }
}

impl IntoElement for Card {
    type Element = AnyElement;

    fn into_element(self) -> Self::Element {
        let (base_bg, hover_bg, base_border, hover_border) = match self.variant {
            CardVariant::Default => (
                colors::card(),
                colors::card(),
                colors::border().opacity(0.6),
                colors::border().opacity(0.6),
            ),
            CardVariant::Interactive => (
                colors::card(),
                mix_color(colors::card(), colors::secondary(), 0.35),
                colors::border().opacity(0.6),
                colors::border(),
            ),
            CardVariant::Outline => (
                colors::card().opacity(0.3),
                colors::card().opacity(0.5),
                colors::border().opacity(0.8),
                colors::border(),
            ),
            CardVariant::Muted => (
                colors::muted().opacity(0.22),
                colors::muted().opacity(0.38),
                colors::border().opacity(0.6),
                colors::border().opacity(0.8),
            ),
            CardVariant::Success => (
                colors::success().opacity(0.08),
                colors::success().opacity(0.14),
                colors::success().opacity(0.5),
                colors::success().opacity(0.8),
            ),
            CardVariant::Warning => (
                colors::card().opacity(0.85),
                colors::card().opacity(0.95),
                colors::warning().opacity(0.5),
                colors::warning().opacity(0.8),
            ),
            CardVariant::Destructive => (
                colors::card().opacity(0.9),
                colors::card().opacity(0.98),
                colors::destructive().opacity(0.5),
                colors::destructive().opacity(0.8),
            ),
        };

        let bg_color = mix_color(base_bg, hover_bg, self.hover_progress);
        let border_color = mix_color(base_border, hover_border, self.hover_progress);

        let mut el = div()
            .w_full()
            .rounded(self.radius.to_pixels())
            .border_1()
            .border_color(border_color)
            .bg(bg_color)
            .overflow_hidden();

        if let Some(h) = self.min_h {
            el = el.min_h(h);
        }

        if self.interactive {
            // Card::interactive is the only constructor that sets this flag and always stores an id.
            let id = self
                .id
                .expect("interactive cards always have a stable element id");
            let mut stateful = el.id(id).cursor_pointer();
            if let Some(hk) = self.hover_key {
                let hk_click = hk.clone();
                stateful = stateful.on_hover(move |hovered, window, cx| {
                    hover_motion::set_hovered(hk.clone(), *hovered, window, cx);
                });
                if let Some(click) = self.on_click {
                    stateful = stateful.on_click(move |event, window, cx| {
                        hover_motion::clear_hover(&hk_click, window, cx);
                        click(event, window, cx);
                    });
                }
            } else if let Some(click) = self.on_click {
                stateful = stateful.on_click(click);
            }
            stateful.children(self.children).into_any_element()
        } else if let Some(id) = self.id {
            el.id(id).children(self.children).into_any_element()
        } else {
            el.children(self.children).into_any_element()
        }
    }
}

/// Icon container used in card headers.
pub fn card_icon(path: &'static str, color: Rgba) -> Div {
    div()
        .size(px(36.))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .border_1()
        .border_color(colors::border().opacity(0.7))
        .bg(colors::muted().opacity(0.25))
        .child(svg().path(path).size_4().text_color(color))
}

pub fn virtual_list_card() -> Div {
    div()
        .h(px(72.))
        .w_full()
        .px_4()
        .py_3()
        .flex()
        .items_center()
        .gap_3()
        .rounded(px(8.))
        .border_1()
        .border_color(colors::border())
        .bg(colors::card())
}

/// Reusable interactive overview card with a toggle switch.
pub fn interactive_module_card(
    title: impl Into<SharedString>,
    description: impl Into<SharedString>,
    icon: (&'static str, Rgba),
    enabled: bool,
    open: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    toggle: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> AnyElement {
    let title = title.into();
    let description = description.into();
    let (icon_path, icon_color) = icon;
    let card_id = SharedString::from(format!("module-{title}"));
    let switch_id = SharedString::from(format!("module-switch-{title}"));
    let switch =
        crate::ui::components::toggle_switch::Switch::new(switch_id, enabled, cx).on_toggle(toggle);

    Card::interactive(card_id, cx)
        .min_h(px(72.))
        .on_click(open)
        .child(
            div()
                .px_4()
                .py_3()
                .flex()
                .items_center()
                .gap_3()
                .child(card_icon(icon_path, icon_color))
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .flex()
                        .items_center()
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
                                        .truncate()
                                        .text_sm()
                                        .font_weight(FontWeight::MEDIUM)
                                        .child(title.clone()),
                                )
                                .child(
                                    div()
                                        .min_w_0()
                                        .truncate()
                                        .text_xs()
                                        .text_color(colors::muted_foreground())
                                        .child(description),
                                ),
                        )
                        .child(
                            svg()
                                .path("icons/chevron-right.svg")
                                .size_4()
                                .text_color(colors::muted_foreground()),
                        ),
                )
                .child(switch),
        )
        .into_element()
}

// Module card and layout helpers
pub fn module_card(header: Div, body: Option<Div>) -> Div {
    div()
        .w_full()
        .rounded(px(8.))
        .border_1()
        .border_color(colors::border().opacity(0.6))
        .bg(colors::card())
        .overflow_hidden()
        .child(header)
        .children(body)
}

pub fn module_header(
    icon: (&'static str, Rgba),
    title: impl Into<SharedString>,
    description: impl Into<SharedString>,
    action: Option<AnyElement>,
    divider: bool,
) -> Div {
    let (icon_path, icon_color) = icon;
    let title = title.into();
    let description = description.into();
    div()
        .min_h(px(72.))
        .p_4()
        .flex()
        .items_center()
        .gap_3()
        .when(divider, |header| {
            header
                .border_b_1()
                .border_color(colors::border().opacity(0.6))
        })
        .child(card_icon(icon_path, icon_color))
        .child(
            div()
                .min_w_0()
                .flex_1()
                .child(div().truncate().text_sm().line_height(px(20.)).child(title))
                .child(
                    div()
                        .mt_1()
                        .text_xs()
                        .line_height(px(16.))
                        .text_color(colors::muted_foreground())
                        .truncate()
                        .child(description),
                ),
        )
        .children(action)
}

pub fn module_header_custom(
    icon: (&'static str, Rgba),
    title: impl IntoElement,
    description: impl Into<SharedString>,
    action: Option<AnyElement>,
    divider: bool,
) -> Div {
    let (icon_path, icon_color) = icon;
    let description = description.into();
    div()
        .min_h(px(72.))
        .p_4()
        .flex()
        .items_center()
        .gap_3()
        .when(divider, |header| {
            header
                .border_b_1()
                .border_color(colors::border().opacity(0.6))
        })
        .child(card_icon(icon_path, icon_color))
        .child(
            div()
                .min_w_0()
                .flex_1()
                .child(div().min_w_0().truncate().child(title))
                .child(
                    div()
                        .mt_1()
                        .text_xs()
                        .line_height(px(16.))
                        .text_color(colors::muted_foreground())
                        .truncate()
                        .child(description),
                ),
        )
        .children(action)
}

pub fn module_icon_colored(path: &'static str, color: Rgba) -> Div {
    card_icon(path, color)
}

pub fn module_icon(path: &'static str) -> Div {
    card_icon(path, colors::muted_foreground())
}

pub fn module_body() -> Div {
    div().p_4().flex().flex_col().gap_4()
}

pub fn module_row(
    title: impl Into<SharedString>,
    description: impl Into<SharedString>,
    control: impl IntoElement,
) -> Div {
    let title = title.into();
    let description = description.into();
    div()
        .min_h(px(36.))
        .flex()
        .items_center()
        .justify_between()
        .gap_4()
        .child(
            div()
                .min_w_0()
                .flex_1()
                .child(div().truncate().text_sm().line_height(px(20.)).child(title))
                .child(
                    div()
                        .mt_1()
                        .text_xs()
                        .line_height(px(16.))
                        .text_color(colors::muted_foreground())
                        .truncate()
                        .child(description),
                ),
        )
        .child(control)
}
