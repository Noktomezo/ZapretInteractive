use gpui::prelude::*;
use gpui::*;

use crate::ui::components::text_input::{TextInput, TextInputState};
use crate::ui::foundation::colors::{self, border, input, muted_foreground};
use crate::ui::foundation::control_style::CONTROL_HEIGHT;

/// Container styles for styled text inputs.
pub fn form_input_container() -> Div {
    div()
        .h(CONTROL_HEIGHT)
        .px_3()
        .rounded_md()
        .border_1()
        .border_color(border().opacity(0.8))
        .bg(input().opacity(if colors::is_dark() { 0.30 } else { 0.92 }))
}

/// Standalone styled text input box without a label.
pub struct FormInput {
    state: Entity<TextInputState>,
    width: Option<Pixels>,
    height: Option<Pixels>,
}

impl FormInput {
    pub fn new(state: &Entity<TextInputState>) -> Self {
        Self {
            state: state.clone(),
            width: None,
            height: None,
        }
    }

    pub fn width(mut self, width: Pixels) -> Self {
        self.width = Some(width);
        self
    }

    pub fn height(mut self, height: Pixels) -> Self {
        self.height = Some(height);
        self
    }
}

impl IntoElement for FormInput {
    type Element = AnyElement;

    fn into_element(self) -> Self::Element {
        let mut el = form_input_container();
        if let Some(w) = self.width {
            el = el.w(w).flex_none();
        }
        if let Some(h) = self.height {
            el = el.h(h);
        }
        el.child(TextInput::new(&self.state)).into_any_element()
    }
}

/// Labeled form field component with optional description and custom control.
pub struct FormField {
    label: SharedString,
    description: Option<SharedString>,
    input_element: AnyElement,
    width: Option<Pixels>,
}

impl FormField {
    pub fn new(label: impl Into<SharedString>, state: &Entity<TextInputState>) -> Self {
        Self {
            label: label.into(),
            description: None,
            input_element: FormInput::new(state).into_any_element(),
            width: None,
        }
    }

    pub fn custom(label: impl Into<SharedString>, input_element: impl IntoElement) -> Self {
        Self {
            label: label.into(),
            description: None,
            input_element: input_element.into_any_element(),
            width: None,
        }
    }

    pub fn description(mut self, desc: impl Into<SharedString>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn width(mut self, width: Pixels) -> Self {
        self.width = Some(width);
        self
    }
}

impl IntoElement for FormField {
    type Element = AnyElement;

    fn into_element(self) -> Self::Element {
        let mut container = div().flex().flex_col().gap_1();

        if let Some(w) = self.width {
            container = container.w(w);
        }

        container
            .child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(muted_foreground())
                    .child(self.label),
            )
            .when_some(self.description, |el, desc| {
                el.child(
                    div()
                        .text_xs()
                        .text_color(muted_foreground().opacity(0.8))
                        .child(desc),
                )
            })
            .child(self.input_element)
            .into_any_element()
    }
}

/// Convenience builder function.
pub fn form_field(label: impl Into<SharedString>, state: &Entity<TextInputState>) -> FormField {
    FormField::new(label, state)
}
