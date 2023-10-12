use std::{
    fmt::{Debug, Display},
    str::FromStr,
};

use derive_more::{From, Into};
use serde::{de::Error, Deserialize, Serialize};
use std::hash::Hash;

use crate::types::{LogicalPixels, Point};

use self::{button::ButtonStyle, computed::ComputedBorderStyle, text_input::TextInputStyle};

pub mod button;
pub mod computed;
pub mod condition;
pub mod defaults;
pub mod text_input;

#[derive(Debug, Clone, Copy, PartialEq, From, Into)]
pub struct Color(tiny_skia::Color);

impl Serialize for Color {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let c = self.0;
        let hex = csscolorparser::Color::new(
            c.red().into(),
            c.green().into(),
            c.blue().into(),
            c.alpha().into(),
        )
        .to_hex_string();
        hex.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let str = <String>::deserialize(deserializer)?;
        let color = csscolorparser::parse(&str).map_err(D::Error::custom)?;
        Ok(Self(
            tiny_skia::Color::from_rgba(
                color.r as f32,
                color.g as f32,
                color.b as f32,
                color.a as f32,
            )
            .ok_or_else(|| D::Error::custom("invalid color"))?,
        ))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Palette {
    pub foreground: Color,
    pub background: Color,
    pub selected_text_color: Color,
    pub selected_text_background: Color,
}

// TODO: remove Debug
pub trait Class: Debug + Clone + Display + FromStr<Err = Self::FromStrErr> {
    type FromStrErr: std::error::Error + Send + Sync + 'static;
}

impl<T> Class for T
where
    T: Debug + Clone + Display + FromStr,
    <T as FromStr>::Err: std::error::Error + Send + Sync + 'static,
{
    type FromStrErr = <T as FromStr>::Err;
}

pub trait ElementState: Eq + Hash + Sized {
    type Class: Class;
    fn all() -> Vec<Self>;
    fn matches(&self, class: &Self::Class) -> bool;
}

pub trait VariantStyle: Default {
    type State: ElementState;
    type Computed;
    fn apply(&mut self, other: &Self);
    fn compute(&self, style: &Style, scale: f32) -> Self::Computed;
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RelativeOffset {
    // from 0 to 1
    pub x: f32,
    pub y: f32,
}

/// A shader spreading mode.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum SpreadMode {
    /// Replicate the edge color if the shader draws outside of its
    /// original bounds.
    Pad,

    /// Repeat the shader's image horizontally and vertically, alternating
    /// mirror images so that adjacent images always seam.
    Reflect,

    /// Repeat the shader's image horizontally and vertically.
    Repeat,
}

impl From<SpreadMode> for tiny_skia::SpreadMode {
    fn from(value: SpreadMode) -> Self {
        match value {
            SpreadMode::Pad => Self::Pad,
            SpreadMode::Reflect => Self::Reflect,
            SpreadMode::Repeat => Self::Repeat,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradientStop {
    pub position: f32,
    pub color: Color,
}

impl GradientStop {
    pub fn new(position: f32, color: Color) -> Self {
        Self { position, color }
    }
}

impl From<GradientStop> for tiny_skia::GradientStop {
    fn from(value: GradientStop) -> Self {
        Self::new(value.position, value.color.into())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinearGradient {
    pub start: RelativeOffset,
    pub end: RelativeOffset,
    pub stops: Vec<GradientStop>,
    pub mode: SpreadMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Background {
    Solid { color: Color },
    LinearGradient(LinearGradient),
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Style {
    pub font: RootFontStyle,
    pub palette: Palette,
    pub text_input: TextInputStyle,
    pub button: ButtonStyle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
                color: color.into(),
                width: width.to_physical(scale),
                radius: radius.to_physical(scale),
            })
        } else {
            None
        }
    }
}
