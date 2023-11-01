use tiny_skia::Color;

use super::OldStyle;

pub fn default_style() -> OldStyle {
    json5::from_str(include_str!("../../themes/default/theme.json5")).unwrap()
}

pub fn text_color() -> Color {
    Color::from_rgba8(0, 0, 0, 255)
}

pub fn selected_text_color() -> Color {
    Color::from_rgba8(255, 255, 255, 255)
}

pub fn selected_text_background() -> Color {
    Color::from_rgba8(100, 100, 150, 255)
}
