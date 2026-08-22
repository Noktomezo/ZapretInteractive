use gpui::prelude::*;
use gpui::*;

#[derive(Clone, Debug)]
pub enum ConfirmTarget {
    DeleteCategory {
        id: String,
        name: String,
    },
    DeleteStrategy {
        category_id: String,
        strategy_id: String,
        name: String,
    },
    DeleteFilter {
        id: String,
        name: String,
    },
    DeletePlaceholder {
        index: usize,
        name: String,
    },
    ResetConfig,
}

impl ConfirmTarget {
    pub fn title(&self) -> std::borrow::Cow<'static, str> {
        match self {
            ConfirmTarget::DeleteCategory { .. } => t!("dialog.delete_category"),
            ConfirmTarget::DeleteStrategy { .. } => t!("dialog.delete_strategy"),
            ConfirmTarget::DeleteFilter { .. } => t!("dialog.delete_filter"),
            ConfirmTarget::DeletePlaceholder { .. } => t!("dialog.delete_placeholder"),
            ConfirmTarget::ResetConfig => t!("dialog.reset_config"),
        }
    }

    pub fn description(&self) -> String {
        match self {
            ConfirmTarget::DeleteCategory { name, .. } => {
                format!("Категория «{name}» и все стратегии внутри неё будут удалены.")
            }
            ConfirmTarget::DeleteStrategy { name, .. } => {
                format!("Стратегия «{name}» будет удалена.")
            }
            ConfirmTarget::DeleteFilter { name, .. } => {
                format!("Файл фильтра «{name}» будет удален.")
            }
            ConfirmTarget::DeletePlaceholder { name, .. } => {
                format!("Плейсхолдер «{name}» будет удален.")
            }
            ConfirmTarget::ResetConfig => {
                "Все настройки, пользовательские стратегии и фильтры будут сброшены к значениям по умолчанию.".into()
            }
        }
    }

    pub fn confirm_label(&self) -> std::borrow::Cow<'static, str> {
        match self {
            ConfirmTarget::ResetConfig => t!("settings.btn_reset"),
            _ => t!("strategies.btn_delete"),
        }
    }
}

pub fn render_confirm_dialog(
    target: &ConfirmTarget,
    closing_progress: Option<f32>,
    on_cancel: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_confirm: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> AnyElement {
    let cancel_btn =
        crate::ui::components::button::Button::new("confirm-cancel", t!("dialog.btn_cancel"), cx)
            .secondary()
            .small()
            .on_click(on_cancel);

    let confirm_btn =
        crate::ui::components::button::Button::new("confirm-action", target.confirm_label(), cx)
            .destructive()
            .small()
            .on_click(on_confirm);

    crate::ui::components::modal_dialog::ModalDialog::new()
        .width(px(420.))
        .anim_id("confirm-dialog-appear")
        .title(target.title())
        .description(target.description())
        .closing_progress(closing_progress)
        .action(cancel_btn)
        .action(confirm_btn)
        .into_any_element()
}
