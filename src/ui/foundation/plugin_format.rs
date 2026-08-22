use gpui::SharedString;

use crate::ui::components::badge::BadgeStyle;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub vst2: usize,
    pub vst3: usize,
    total: usize,
}

impl Counts {
    pub const fn total(self) -> usize {
        self.total
    }
}

pub fn counts<'a>(formats: impl IntoIterator<Item = &'a str>) -> Counts {
    let mut counts = Counts::default();
    for format in formats {
        counts.total += 1;
        if format.eq_ignore_ascii_case("VST") || format.eq_ignore_ascii_case("VST2") {
            counts.vst2 += 1;
        } else if format.eq_ignore_ascii_case("VST3") {
            counts.vst3 += 1;
        }
    }
    counts
}

pub fn display_name(format: &str) -> SharedString {
    if format.eq_ignore_ascii_case("VST") {
        "VST2".into()
    } else {
        format.to_uppercase().into()
    }
}

pub fn badge_style(format: &str) -> BadgeStyle {
    if format.eq_ignore_ascii_case("VST") || format.eq_ignore_ascii_case("VST2") {
        BadgeStyle::Cyan
    } else {
        BadgeStyle::Purple
    }
}

#[cfg(test)]
mod tests {
    use super::{badge_style, counts, display_name};
    use crate::ui::components::badge::BadgeStyle;

    #[test]
    fn distinguishes_juces_legacy_vst_name() {
        assert_eq!(display_name("VST"), "VST2");
        assert_eq!(display_name("VST3"), "VST3");
    }

    #[test]
    fn gives_vst2_a_distinct_badge_color() {
        assert!(matches!(badge_style("VST"), BadgeStyle::Cyan));
        assert!(matches!(badge_style("VST2"), BadgeStyle::Cyan));
        assert!(matches!(badge_style("VST3"), BadgeStyle::Purple));
    }

    #[test]
    fn counts_juce_vst_as_vst2() {
        let counts = counts(["VST", "vst2", "VST3", "Unknown"]);

        assert_eq!(counts.vst2, 2);
        assert_eq!(counts.vst3, 1);
        assert_eq!(counts.total(), 4);
    }
}
