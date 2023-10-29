use std::{
    borrow::{Borrow, Cow},
    collections::BTreeMap,
    fmt::{Debug, Display},
    path::Path,
    str::FromStr,
};

use anyhow::Result;
use derive_more::{From, Into};
use lightningcss::{
    properties::Property, rules::CssRule, selector::Selector, stylesheet::StyleSheet,
};
use log::warn;
use serde::{de::Error, Deserialize, Serialize};
use std::hash::Hash;

use crate::{
    style::css::replace_vars,
    types::{LogicalPixels, Point},
};

use self::{
    button::ButtonStyle,
    computed::{ComputedBackground, ComputedBorderStyle, ComputedLinearGradient},
    text_input::TextInputStyle,
};

pub mod button;
pub mod computed;
pub mod condition;
mod css;
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

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, From, Into,
)]
pub struct ColorRef(Cow<'static, str>);

#[allow(non_upper_case_globals)]
impl ColorRef {
    pub const foreground: Self = Self(Cow::Borrowed("foreground"));
    pub const background: Self = Self(Cow::Borrowed("background"));
    pub const selected_text_color: Self = Self(Cow::Borrowed("selected_text_color"));
    pub const selected_text_background: Self = Self(Cow::Borrowed("selected_text_background"));
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Palette(BTreeMap<ColorRef, Color>);

impl Palette {
    pub fn get(&self, color: impl Borrow<ColorRef>) -> tiny_skia::Color {
        if let Some(c) = self.0.get(color.borrow()) {
            (*c).into()
        } else {
            warn!("missing color in palette: {:?}", color.borrow());
            self.0
                .get(&ColorRef::foreground)
                .map(|c| (*c).into())
                .unwrap_or(tiny_skia::Color::BLACK)
        }
    }
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
    fn compute(&self, style: &OldStyle, scale: f32) -> Self::Computed;
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
    pub color: ColorRef,
}

impl GradientStop {
    pub fn new(position: f32, color: ColorRef) -> Self {
        Self { position, color }
    }

    pub fn compute(&self, palette: &Palette) -> tiny_skia::GradientStop {
        tiny_skia::GradientStop::new(self.position, palette.get(&self.color))
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
    Solid { color: ColorRef },
    LinearGradient(LinearGradient),
}

impl Background {
    pub fn compute(&self, palette: &Palette) -> ComputedBackground {
        match self {
            Background::Solid { color } => ComputedBackground::Solid {
                color: palette.get(color),
            },
            Background::LinearGradient(g) => {
                ComputedBackground::LinearGradient(ComputedLinearGradient {
                    start: g.start,
                    end: g.end,
                    stops: g
                        .stops
                        .iter()
                        .map(|g| tiny_skia::GradientStop::new(g.position, palette.get(&g.color)))
                        .collect(),
                    mode: g.mode.into(),
                })
            }
        }
    }
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
pub struct OldStyle {
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
    pub width: Option<LogicalPixels>,
    pub color: Option<ColorRef>,
    pub radius: Option<LogicalPixels>,
}

impl BorderStyle {
    pub fn apply(&mut self, other: &Self) {
        if let Some(color) = &other.color {
            self.color = Some(color.clone());
        }
        if let Some(radius) = other.radius {
            self.radius = Some(radius);
        }
    }

    pub fn to_physical(&self, scale: f32, palette: &Palette) -> ComputedBorderStyle {
        ComputedBorderStyle {
            color: self
                .color
                .as_ref()
                .map_or(tiny_skia::Color::TRANSPARENT, |color| palette.get(color)),
            radius: self.radius.unwrap_or_default().to_physical(scale),
        }
    }
}

pub struct Style {
    pub css: StyleSheet<'static, 'static>,
    // TODO: allow to bundle style and referenced files
}

impl Style {
    pub fn load(path: impl AsRef<Path>) -> Result<Style> {
        let data = std::fs::read_to_string(path).unwrap();
        let mut style = StyleSheet::parse(&data, Default::default()).unwrap();
        replace_vars(&mut style);
        let code = style.to_css(Default::default()).unwrap().code;
        let style = StyleSheet::parse(&code, Default::default()).unwrap();
        // There is no StyleSheet::into_owned, so we have to use serialization :(
        let serialized = serde_value::to_value(&style)?;

        println!("{style:#?}");
        Ok(Style {
            css: serialized.deserialize_into()?,
        })
    }

    // TODO: use specificality and !important to sort from low to high priority
    pub fn find_rules(
        &self,
        check_selector: impl Fn(&Selector) -> bool,
    ) -> Vec<&Property<'static>> {
        let mut results = Vec::new();
        for rule in &self.css.rules.0 {
            if let CssRule::Style(rule) = rule {
                for selector in &rule.selectors.0 {
                    if check_selector(selector) {
                        results.extend(rule.declarations.iter().map(|(dec, _important)| dec));
                    }
                }
            }
        }
        results
    }
}
