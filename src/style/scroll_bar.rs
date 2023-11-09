use anyhow::Result;

use super::{button, FontStyle, Style};

#[derive(Debug, Clone)]
pub struct ComputedStyle {
    pub scroll_left: button::ComputedStyle,
}

impl ComputedStyle {
    pub fn new(style: &Style, scale: f32, root_font: &FontStyle) -> Result<ComputedStyle> {
        Ok(Self {
            scroll_left: button::ComputedStyle::new(style, scale, root_font, Some("scroll_left"))?,
        })
    }
}
