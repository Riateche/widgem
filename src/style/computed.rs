use anyhow::Result;
use log::warn;
use tiny_skia::{Color, GradientStop, SpreadMode};

use crate::types::PhysicalPixels;

use super::{
    button,
    css::{convert_background_color, convert_font, is_root},
    text_input, RelativeOffset, Style,
};

#[derive(Debug, Clone)]
pub struct ComputedBorderStyle {
    pub width: PhysicalPixels,
    pub color: Color,
    pub radius: PhysicalPixels,
}

impl Default for ComputedBorderStyle {
    fn default() -> Self {
        Self {
            width: Default::default(),
            color: Color::TRANSPARENT,
            radius: Default::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ComputedStyle {
    pub scale: f32,

    pub background: Color,
    pub font_metrics: cosmic_text::Metrics,
    pub text_input: text_input::ComputedStyle,
    pub button: button::ComputedStyle,
}

impl ComputedStyle {
    #[allow(dead_code, unused)]
    pub fn new(style: &Style, scale: f32) -> Result<Self> {
        let root_properties = style.find_rules(is_root);
        let background = convert_background_color(&root_properties)?;
        let font = convert_font(&root_properties, None)?;

        Ok(Self {
            scale,
            background: background.unwrap_or_else(|| {
                warn!("missing root background color");
                Color::WHITE
            }),
            font_metrics: font.to_metrics(scale),
            text_input: text_input::ComputedStyle::new(style, scale, &font)?,
            button: button::ComputedStyle::new(style, scale, &font)?,
        })
    }
}

#[derive(Debug, Clone)]
pub enum ComputedBackground {
    Solid { color: Color },
    LinearGradient(ComputedLinearGradient),
}

#[derive(Debug, Clone)]
pub struct ComputedLinearGradient {
    pub start: RelativeOffset,
    pub end: RelativeOffset,
    pub stops: Vec<GradientStop>,
    pub mode: SpreadMode,
}
