use gpui::{FontWeight, Pixels, Styled, px};

pub const CONTROL_FONT_FAMILY: &str = "IBM Plex Sans";
pub const DROPDOWN_CONTROL_HEIGHT: Pixels = px(36.0);
pub const DROPDOWN_CONTROL_WIDTH: Pixels = px(160.0);
pub const DROPDOWN_LABEL_WIDTH: Pixels = px(113.0);
pub const DROPDOWN_ICON_LABEL_WIDTH: Pixels = px(89.0);
pub const DROPDOWN_MENU_LABEL_WIDTH: Pixels = px(144.0);
pub const DROPDOWN_MENU_ICON_LABEL_WIDTH: Pixels = px(120.0);
pub const DROPDOWN_TRAILING_GUTTER: Pixels = px(15.0);

pub trait ControlTypography: Styled + Sized {
    fn control_text(self) -> Self {
        self.font_family(CONTROL_FONT_FAMILY)
            .text_size(px(12.0))
            .font_weight(FontWeight::MEDIUM)
    }
}

impl<T: Styled> ControlTypography for T {}

#[cfg(test)]
mod tests {
    use gpui::px;

    use super::{
        DROPDOWN_CONTROL_WIDTH, DROPDOWN_ICON_LABEL_WIDTH, DROPDOWN_LABEL_WIDTH,
        DROPDOWN_MENU_ICON_LABEL_WIDTH, DROPDOWN_MENU_LABEL_WIDTH, DROPDOWN_TRAILING_GUTTER,
    };

    #[test]
    fn dropdown_gutter_keeps_fades_clear_of_trailing_icons() {
        let horizontal_padding = px(16.0);
        let trailing_icon = px(16.0);
        let leading_icon_and_gap = px(24.0);

        assert_eq!(
            DROPDOWN_LABEL_WIDTH + DROPDOWN_TRAILING_GUTTER + trailing_icon + horizontal_padding,
            DROPDOWN_CONTROL_WIDTH
        );
        assert_eq!(
            DROPDOWN_ICON_LABEL_WIDTH
                + leading_icon_and_gap
                + DROPDOWN_TRAILING_GUTTER
                + trailing_icon
                + horizontal_padding,
            DROPDOWN_CONTROL_WIDTH
        );
        assert_eq!(
            DROPDOWN_MENU_LABEL_WIDTH + horizontal_padding,
            DROPDOWN_CONTROL_WIDTH
        );
        assert_eq!(
            DROPDOWN_MENU_ICON_LABEL_WIDTH + leading_icon_and_gap + horizontal_padding,
            DROPDOWN_CONTROL_WIDTH
        );
    }
}
