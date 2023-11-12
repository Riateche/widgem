use tiny_skia::Color;

use super::Style;

pub fn default_style() -> Style {
    Style::load_bundled(
        include_str!("../../themes/default/theme.css"),
        [
            (
                "scroll_left.svg",
                &include_bytes!("../../themes/default/scroll_left.svg")[..],
            ),
            (
                "scroll_right.svg",
                &include_bytes!("../../themes/default/scroll_right.svg")[..],
            ),
            (
                "scroll_up.svg",
                &include_bytes!("../../themes/default/scroll_up.svg")[..],
            ),
            (
                "scroll_down.svg",
                &include_bytes!("../../themes/default/scroll_down.svg")[..],
            ),
            (
                "scroll_grip_x.svg",
                &include_bytes!("../../themes/default/scroll_grip_x.svg")[..],
            ),
            (
                "scroll_grip_y.svg",
                &include_bytes!("../../themes/default/scroll_grip_y.svg")[..],
            ),
        ],
    )
    .unwrap()
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

pub const DEFAULT_PREFERRED_WIDTH_EM: f32 = 10.0;
pub const DEFAULT_MIN_WIDTH_EM: f32 = 2.0;

pub const DEFAULT_LINE_HEIGHT: f32 = 1.2;
