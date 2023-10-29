#![allow(clippy::single_match)]

use std::collections::HashMap;

use anyhow::{bail, Context, Result};
use lightningcss::{
    properties::{
        font::{FontSize, LineHeight},
        Property,
    },
    values::{
        color::CssColor,
        length::{LengthPercentage, LengthValue},
        percentage::DimensionPercentage,
    },
};
use log::warn;
use tiny_skia::{Color, GradientStop, SpreadMode};

use crate::types::{LogicalPixels, LpxSuffix, PhysicalPixels};

use super::{
    button, condition::ClassRules, css::is_root, text_input, ColorRef, ElementState, OldStyle,
    RelativeOffset, RootFontStyle, Style, VariantStyle,
};

#[derive(Debug, Clone)]
pub struct ComputedStyleVariants<T: VariantStyle>(HashMap<T::State, T::Computed>);

const DEFAULT_LINE_HEIGHT: f32 = 1.2;

impl<T: VariantStyle> ComputedStyleVariants<T> {
    pub fn new(rules: &ClassRules<T>, style: &OldStyle, scale: f32) -> Self {
        let mut map = HashMap::new();
        for variant in T::State::all() {
            let computed = rules.get(&variant).compute(style, scale);
            map.insert(variant, computed);
        }

        Self(map)
    }

    pub fn get(&self, state: &T::State) -> &T::Computed {
        self.0.get(state).expect("unexpected state")
    }
}

#[derive(Debug, Clone)]
pub struct ComputedBorderStyle {
    pub color: Color,
    pub radius: PhysicalPixels,
}

#[derive(Debug, Clone)]
pub struct ComputedStyle {
    pub scale: f32,

    pub background: Color,
    pub font_metrics: cosmic_text::Metrics,
    pub text_input: text_input::ComputedStyle,
    pub button: button::ComputedStyle,
}

fn convert_color(color: &CssColor) -> Result<Color> {
    if let CssColor::RGBA(color) = color {
        Ok(Color::from_rgba8(
            color.red,
            color.green,
            color.blue,
            color.alpha,
        ))
    } else {
        bail!("unsupported color, use rgb: {color:?}")
    }
}

fn convert_length(value: &LengthValue) -> Result<LogicalPixels> {
    if let LengthValue::Px(size) = value {
        Ok(size.lpx())
    } else {
        bail!("unsupported value, use px: {value:?}");
    }
}

#[allow(clippy::collapsible_match)]
fn convert_font_size(size: &FontSize) -> Result<LogicalPixels> {
    if let FontSize::Length(size) = size {
        if let LengthPercentage::Dimension(size) = size {
            return convert_length(size);
        }
    }
    bail!("unsupported font size, use px: {size:?}");
}

fn convert_line_height(value: &LineHeight, font_size: LogicalPixels) -> Result<LogicalPixels> {
    match value {
        LineHeight::Normal => Ok(font_size * DEFAULT_LINE_HEIGHT),
        LineHeight::Number(value) => Ok(font_size * *value),
        LineHeight::Length(value) => match value {
            DimensionPercentage::Dimension(value) => convert_length(value),
            DimensionPercentage::Percentage(value) => Ok(font_size * value.0),
            DimensionPercentage::Calc(_) => bail!("calc is unsupported"),
        },
    }
}

fn convert_font(
    properties: &[&Property<'static>],
    root: Option<RootFontStyle>,
) -> Result<RootFontStyle> {
    let mut font_size = None;
    let mut line_height = None;
    for property in properties {
        match property {
            Property::FontSize(size) => {
                font_size = Some(convert_font_size(size)?);
            }
            Property::Font(font) => {
                font_size = Some(convert_font_size(&font.size)?);
            }
            _ => {}
        }
    }

    let font_size = font_size
        .or_else(|| root.map(|root| root.font_size))
        .context("missing root font size")?;

    for property in properties {
        match property {
            Property::LineHeight(value) => {
                line_height = Some(convert_line_height(value, font_size)?);
            }
            _ => {}
        }
    }

    let line_height = line_height.unwrap_or_else(|| font_size * DEFAULT_LINE_HEIGHT);

    Ok(RootFontStyle {
        font_size,
        line_height,
    })
}

impl ComputedStyle {
    #[allow(dead_code, unused)]
    pub fn new(style: &Style, scale: f32) -> Result<Self> {
        let mut background = None;
        let root_properties = style.find_rules(is_root);
        for property in &root_properties {
            match property {
                Property::Background(backgrounds) => {
                    if backgrounds.is_empty() {
                        warn!("empty vec in Property::Background");
                        continue;
                    }
                    if backgrounds.len() > 1 {
                        warn!("multiple backgrounds are not supported");
                    }
                    background = Some(convert_color(&backgrounds[0].color)?);
                }
                Property::BackgroundColor(color) => {
                    background = Some(convert_color(color)?);
                }
                _ => {}
            }
        }

        let font = convert_font(&root_properties, None)?;

        Ok(Self {
            scale,
            background: background.unwrap_or_else(|| {
                warn!("missing root background color");
                Color::WHITE
            }),
            font_metrics: font.to_metrics(scale),
            text_input: todo!(),
            button: todo!(),
        })
    }

    pub fn old_new(style: OldStyle, scale: f32) -> Self {
        Self {
            font_metrics: style.font.to_metrics(scale),
            text_input: text_input::ComputedStyle::new(&style, scale),
            button: button::ComputedStyle::new(&style, scale),
            background: style.palette.get(&ColorRef::background),
            scale,
        }
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
