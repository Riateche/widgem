use std::fmt::Debug;

use std::hash::Hash;
use tiny_skia::Color;

pub mod button;
pub mod computed;
pub mod condition;
pub mod defaults;
pub mod text_input;

use crate::types::{LogicalPixels, Point};

use self::{button::ButtonStyle, computed::ComputedBorderStyle, text_input::TextInputStyle};

#[derive(Debug, Clone)]
pub struct Palette {
    pub foreground: Color,
    pub background: Color,
    pub selected_text_color: Color,
    pub selected_text_background: Color,
}

pub trait ElementState: Eq + Hash + Sized {
    type Class: Debug + Clone; // TODO: remove Debug
    fn all() -> Vec<Self>;
    fn matches(&self, class: &Self::Class) -> bool;
}

pub trait VariantStyle: Default {
    type State: ElementState;
    type Computed;
    fn apply(&mut self, other: &Self);
    fn compute(&self, style: &Style, scale: f32) -> Self::Computed;
}

#[derive(Debug, Clone)]
pub enum Background {
    Solid(Color),
    LinearGradient(()), // TODO
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Padding {
    pub x: LogicalPixels,
    pub y: LogicalPixels,
}

impl Padding {
    pub fn new(x: LogicalPixels, y: LogicalPixels) -> Self {
        Self { x, y }
    }
    pub fn to_physical(self, scale: f32) -> Point {
        Point {
            x: self.x.to_physical(scale).get(),
            y: self.y.to_physical(scale).get(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Style {
    pub font: RootFontStyle,
    pub palette: Palette,
    pub text_input: TextInputStyle,
    pub button: ButtonStyle,
}

#[derive(Debug, Clone)]
pub struct RootFontStyle {
    pub font_size: LogicalPixels,
    pub line_height: LogicalPixels,
    // TODO: font family, attributes, etc.
}

impl RootFontStyle {
    pub fn apply(&mut self, other: &FontStyle) {
        if let Some(font_size) = other.font_size {
            self.font_size = font_size;
        }
        if let Some(line_height) = other.line_height {
            self.line_height = line_height;
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct FontStyle {
    pub font_size: Option<LogicalPixels>,
    pub line_height: Option<LogicalPixels>,
    // TODO: font family, attributes, etc.
}

impl FontStyle {
    pub fn apply(&mut self, other: &Self) {
        if let Some(font_size) = other.font_size {
            self.font_size = Some(font_size);
        }
        if let Some(line_height) = other.line_height {
            self.line_height = Some(line_height);
        }
    }
}

impl RootFontStyle {
    pub fn to_metrics(&self, scale: f32) -> cosmic_text::Metrics {
        cosmic_text::Metrics {
            font_size: self.font_size.to_physical(scale).get() as f32,
            line_height: self.line_height.to_physical(scale).get() as f32,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct BorderStyle {
    pub color: Option<Color>,
    pub width: Option<LogicalPixels>,
    pub radius: Option<LogicalPixels>,
}

impl BorderStyle {
    pub fn apply(&mut self, other: &Self) {
        if let Some(color) = other.color {
            self.color = Some(color);
        }
        if let Some(width) = other.width {
            self.width = Some(width);
        }
        if let Some(radius) = other.radius {
            self.radius = Some(radius);
        }
    }

    pub fn to_physical(&self, scale: f32) -> Option<ComputedBorderStyle> {
        if let (Some(color), Some(width)) = (self.color, self.width) {
            if width.get() == 0 {
                return None;
            }
            let radius = self.radius.unwrap_or_default();
            Some(ComputedBorderStyle {
                color,
                width: width.to_physical(scale),
                radius: radius.to_physical(scale),
            })
        } else {
            None
        }
    }
}
