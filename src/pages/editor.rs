use std::time::Instant;

use super::*;
use crate::ui::components::button::{ButtonVariant, button};
use crate::ui::components::form_field::form_field;
use crate::ui::components::text_area::TextArea;

impl AppView {
    pub fn close_editor(&mut self, cx: &mut Context<Self>) {
        if let Some(target) = self.editor.take() {
            self.closing_editor = Some((target, Instant::now()));
            cx.notify();
        }
    }

    pub fn render_editor(
        &mut self,
        closing_progress: Option<f32>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let target = self
            .editor
            .clone()
            .or_else(|| self.closing_editor.as_ref().map(|(t, _)| t.clone()));
        let Some(target) = target else {
            return div().into_any_element();
        };
        let title = match target {
            EditorTarget::Category(_) => t!("strategies.category_name"),
            EditorTarget::Strategy { .. } => t!("strategies.strategy_name"),
            EditorTarget::Placeholder(_) => t!("placeholders.placeholder_name"),
            EditorTarget::Filter(_) => t!("filters.filter_name"),
        };
        let root = cx.entity().downgrade();
        let cancel_btn = button(
            "editor-cancel",
            t!("dialog.btn_cancel"),
            ButtonVariant::Secondary,
            cx.listener(|this, _, _, cx| {
                this.close_editor(cx);
            }),
            cx,
        );
        let save_btn = button(
            "editor-save",
            t!("dialog.btn_save"),
            ButtonVariant::Primary,
            move |_, _, cx| {
                if let Some(root) = root.upgrade() {
                    root.update(cx, |this, cx| this.save_editor(cx));
                }
            },
            cx,
        );

        let mut dialog = crate::ui::components::modal_dialog::ModalDialog::new()
            .width(px(520.))
            .max_height(px(600.))
            .anim_id("editor-dialog-appear")
            .title(title)
            .closing_progress(closing_progress)
            .child(form_field(
                t!("strategies.category_name"),
                &self.primary_input,
            ));

        if matches!(
            target,
            EditorTarget::Placeholder(_) | EditorTarget::Filter(_)
        ) {
            let label = match target {
                EditorTarget::Placeholder(_) => t!("placeholders.placeholder_value"),
                _ => t!("filters.filter_name"),
            };
            dialog = dialog.child(form_field(label, &self.secondary_input));
        }

        if matches!(target, EditorTarget::Strategy { .. }) {
            dialog = dialog.child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .text_color(muted_foreground())
                            .child(t!("strategies.cli_arguments")),
                    )
                    .child(TextArea::new(&self.text_area)),
            );
        }

        if matches!(target, EditorTarget::Filter(_)) {
            dialog = dialog.child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .text_color(muted_foreground())
                            .child(t!("filters.filter_expression")),
                    )
                    .child(TextArea::new(&self.text_area)),
            );
        }

        dialog
            .action(cancel_btn)
            .action(save_btn)
            .into_any_element()
    }

    fn set_editor_values(
        &mut self,
        primary: String,
        secondary: String,
        text_area_content: String,
        cx: &mut Context<Self>,
    ) {
        self.primary_input
            .update(cx, |input, cx| input.set_value(primary, cx));
        self.secondary_input
            .update(cx, |input, cx| input.set_value(secondary, cx));
        self.text_area
            .update(cx, |area, cx| area.set_value(text_area_content, cx));
    }

    pub(crate) fn open_category(&mut self, id: Option<String>, cx: &mut Context<Self>) {
        let name = id
            .as_ref()
            .and_then(|id| {
                self.state
                    .read(cx)
                    .config
                    .categories
                    .iter()
                    .find(|item| &item.id == id)
            })
            .map(|item| item.name.clone())
            .unwrap_or_default();
        self.closing_editor = None;
        self.editor = Some(EditorTarget::Category(id));
        self.set_editor_values(name, String::new(), String::new(), cx);
    }

    pub(crate) fn open_strategy(
        &mut self,
        category_id: String,
        strategy_id: Option<String>,
        cx: &mut Context<Self>,
    ) {
        let item = strategy_id
            .as_ref()
            .and_then(|id| {
                self.state
                    .read(cx)
                    .config
                    .categories
                    .iter()
                    .find(|item| item.id == category_id)?
                    .strategies
                    .iter()
                    .find(|item| &item.id == id)
            })
            .cloned();
        self.closing_editor = None;
        self.editor = Some(EditorTarget::Strategy {
            category_id,
            strategy_id,
        });
        self.set_editor_values(
            item.as_ref()
                .map(|item| item.name.clone())
                .unwrap_or_default(),
            String::new(),
            item.map(|item| item.content).unwrap_or_default(),
            cx,
        );
    }

    pub(crate) fn open_placeholder(&mut self, index: Option<usize>, cx: &mut Context<Self>) {
        let item = index
            .and_then(|index| self.state.read(cx).config.placeholders.get(index))
            .cloned();
        self.closing_editor = None;
        self.editor = Some(EditorTarget::Placeholder(index));
        self.set_editor_values(
            item.as_ref()
                .map(|item| item.name.clone())
                .unwrap_or_default(),
            item.map(|item| item.path).unwrap_or_default(),
            String::new(),
            cx,
        );
    }

    pub(crate) fn open_filter(&mut self, id: Option<String>, cx: &mut Context<Self>) {
        let item = id
            .as_ref()
            .and_then(|id| {
                self.state
                    .read(cx)
                    .config
                    .filters
                    .iter()
                    .find(|item| &item.id == id)
            })
            .cloned();
        self.closing_editor = None;
        self.editor = Some(EditorTarget::Filter(id));
        self.set_editor_values(
            item.as_ref()
                .map(|item| item.name.clone())
                .unwrap_or_default(),
            item.as_ref()
                .map(|item| item.filename.clone())
                .unwrap_or_default(),
            item.map(|item| item.content).unwrap_or_default(),
            cx,
        );
    }

    fn save_editor(&mut self, cx: &mut Context<Self>) {
        let Some(target) = self.editor.take() else {
            return;
        };
        let name = self.primary_input.read(cx).value().trim().to_owned();
        let second = self.secondary_input.read(cx).value().to_owned();
        let multiline = self.text_area.read(cx).value().to_owned();
        if name.is_empty() {
            self.editor = Some(target);
            return;
        }
        self.state.update(cx, |state, cx| match target.clone() {
            EditorTarget::Category(id) => state.save_category(id.as_deref(), name, cx),
            EditorTarget::Strategy {
                category_id,
                strategy_id,
            } => state.save_strategy(&category_id, strategy_id.as_deref(), name, multiline, cx),
            EditorTarget::Placeholder(index) => state.save_placeholder(index, name, second, cx),
            EditorTarget::Filter(id) => {
                state.save_filter(id.as_deref(), name, second, multiline, cx)
            }
        });
        self.closing_editor = Some((target, Instant::now()));
        cx.notify();
    }
}
