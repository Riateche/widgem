#![allow(clippy::single_match)]

use std::collections::HashMap;

use anyhow::{bail, Context, Result};
use lightningcss::{
    properties::{
        border::{BorderSideWidth, LineStyle},
        font::{FontSize, LineHeight},
        size::Size,
        Property,
    },
    values::{
        color::CssColor,
        gradient::{Gradient, GradientItem, LineDirection, LinearGradient},
        image::Image,
        length::{Length, LengthPercentage, LengthPercentageOrAuto, LengthValue},
        percentage::DimensionPercentage,
        position::{HorizontalPositionKeyword, VerticalPositionKeyword},
    },
};
use log::warn;
use tiny_skia::{Color, GradientStop, SpreadMode};

use crate::types::{LogicalPixels, LpxSuffix, PhysicalPixels, Point};

use super::{
    button, condition::ClassRules, css::is_root, text_input, ElementState, OldStyle,
    RelativeOffset, RootFontStyle, Style, VariantStyle,
};

#[derive(Debug, Clone)]
pub struct ComputedStyleVariants<T: VariantStyle>(pub(crate) HashMap<T::State, T::Computed>);

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

fn convert_length(value: &LengthValue, font_size: Option<LogicalPixels>) -> Result<LogicalPixels> {
    match value {
        LengthValue::Px(size) => Ok(size.lpx()),
        LengthValue::Em(size) => {
            if let Some(font_size) = font_size {
                Ok(font_size * *size)
            } else {
                bail!("unsupported value (em), font size is unknown");
            }
        }
        _ => {
            bail!("unsupported value, use px: {value:?}");
        }
    }
}

#[allow(clippy::collapsible_match)]
fn convert_font_size(size: &FontSize) -> Result<LogicalPixels> {
    if let FontSize::Length(size) = size {
        if let LengthPercentage::Dimension(size) = size {
            return convert_length(size, None);
        }
    }
    bail!("unsupported font size, use px: {size:?}");
}

fn convert_dimension_percentage(
    value: &DimensionPercentage<LengthValue>,
    total: Option<LogicalPixels>,
    font_size: Option<LogicalPixels>,
) -> Result<LogicalPixels> {
    match value {
        DimensionPercentage::Dimension(value) => convert_length(value, font_size),
        DimensionPercentage::Percentage(value) => {
            if let Some(total) = total {
                Ok(total * value.0)
            } else {
                bail!("percentage is unsupported in this context");
            }
        }
        DimensionPercentage::Calc(_) => bail!("calc is unsupported"),
    }
}

fn convert_line_height(value: &LineHeight, font_size: LogicalPixels) -> Result<LogicalPixels> {
    match value {
        LineHeight::Normal => Ok(font_size * DEFAULT_LINE_HEIGHT),
        LineHeight::Number(value) => Ok(font_size * *value),
        LineHeight::Length(value) => {
            convert_dimension_percentage(value, Some(font_size), Some(font_size))
        }
    }
}

// TODO: pass root properties instead?
pub fn convert_font(
    properties: &[&Property<'static>],
    root: Option<&RootFontStyle>,
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

pub fn convert_main_color(properties: &[&Property<'static>]) -> Result<Option<Color>> {
    let mut color = None;
    for property in properties {
        match property {
            Property::Color(value) => {
                color = Some(convert_color(value)?);
            }
            _ => {}
        }
    }
    Ok(color)
}

fn convert_single_padding(
    value: &LengthPercentageOrAuto,
    font_size: LogicalPixels,
) -> Result<LogicalPixels> {
    match value {
        LengthPercentageOrAuto::Auto => Ok(0.0.into()),
        LengthPercentageOrAuto::LengthPercentage(value) => {
            if let LengthPercentage::Dimension(value) = value {
                convert_length(value, Some(font_size))
            } else {
                bail!("unsupported value ({value:?})")
            }
        }
    }
}

pub fn convert_padding(
    properties: &[&Property<'static>],
    scale: f32,
    font_size: LogicalPixels,
) -> Result<Point> {
    let mut left = None;
    let mut top = None;
    for property in properties {
        match property {
            Property::Padding(value) => {
                left = Some(convert_single_padding(&value.left, font_size)?);
                top = Some(convert_single_padding(&value.top, font_size)?);
            }
            Property::PaddingLeft(value) => {
                left = Some(convert_single_padding(value, font_size)?);
            }
            Property::PaddingTop(value) => {
                top = Some(convert_single_padding(value, font_size)?);
            }
            _ => {}
        }
    }
    Ok(Point::new(
        left.unwrap_or_default().to_physical(scale).get(),
        top.unwrap_or_default().to_physical(scale).get(),
    ))
}

pub fn convert_width(
    properties: &[&Property<'static>],
    scale: f32,
    font_size: LogicalPixels,
) -> Result<Option<PhysicalPixels>> {
    let mut width = None;
    for property in properties {
        match property {
            Property::Width(value) => match value {
                Size::Auto => {}
                Size::LengthPercentage(value) => {
                    width = Some(convert_dimension_percentage(value, None, Some(font_size))?);
                }
                _ => warn!("unsupported width value: {value:?}"),
            },
            _ => {}
        }
    }
    Ok(width.map(|width| width.to_physical(scale)))
}

fn convert_border_width(width: &BorderSideWidth) -> Result<LogicalPixels> {
    if let BorderSideWidth::Length(width) = width {
        match width {
            Length::Value(width) => convert_length(width, None),
            Length::Calc(_) => bail!("calc is unsupported"),
        }
    } else {
        bail!("unsupported border width (use explicit width): {width:?}");
    }
}

pub fn convert_border(
    properties: &[&Property<'static>],
    scale: f32,
    text_color: Color,
) -> Result<ComputedBorderStyle> {
    let mut width = None;
    let mut color = None;
    let mut radius = None;
    let mut style = LineStyle::None;
    for property in properties {
        match property {
            Property::Border(value) => {
                width = Some(convert_border_width(&value.width)?);
                color = Some(convert_color(&value.color)?);
                style = value.style;
            }
            Property::BorderWidth(value) => {
                // TODO: support different sides
                width = Some(convert_border_width(&value.top)?);
            }
            Property::BorderColor(value) => {
                color = Some(convert_color(&value.top)?);
            }
            Property::BorderStyle(value) => {
                style = value.top;
            }
            Property::BorderRadius(value, _prefix) => {
                radius = Some(convert_dimension_percentage(&value.top_left.0, None, None)?);
            }
            _ => {}
        }
    }

    match style {
        LineStyle::None => Ok(ComputedBorderStyle::default()),
        LineStyle::Solid => Ok(ComputedBorderStyle {
            width: width.unwrap_or_default().to_physical(scale),
            color: color.unwrap_or(text_color),
            radius: radius.unwrap_or_default().to_physical(scale),
        }),
        _ => bail!("unsupported border line style: {style:?}"),
    }
}

fn convert_linear_gradient(value: &LinearGradient) -> Result<ComputedLinearGradient> {
    let (start, end) = match value.direction {
        LineDirection::Angle(_) => bail!("angle in unsupported in gradient"),
        LineDirection::Horizontal(value) => match value {
            HorizontalPositionKeyword::Left => {
                (RelativeOffset::new(0.0, 0.0), RelativeOffset::new(1.0, 0.0))
            }
            HorizontalPositionKeyword::Right => {
                (RelativeOffset::new(1.0, 0.0), RelativeOffset::new(0.0, 0.0))
            }
        },
        LineDirection::Vertical(value) => match value {
            VerticalPositionKeyword::Top => {
                (RelativeOffset::new(0.0, 1.0), RelativeOffset::new(0.0, 0.0))
            }
            VerticalPositionKeyword::Bottom => {
                (RelativeOffset::new(0.0, 0.0), RelativeOffset::new(0.0, 1.0))
            }
        },
        LineDirection::Corner {
            horizontal,
            vertical,
        } => match (horizontal, vertical) {
            (HorizontalPositionKeyword::Left, VerticalPositionKeyword::Top) => {
                (RelativeOffset::new(1.0, 1.0), RelativeOffset::new(0.0, 0.0))
            }
            (HorizontalPositionKeyword::Right, VerticalPositionKeyword::Top) => {
                (RelativeOffset::new(0.0, 1.0), RelativeOffset::new(1.0, 0.0))
            }
            (HorizontalPositionKeyword::Left, VerticalPositionKeyword::Bottom) => {
                (RelativeOffset::new(1.0, 0.0), RelativeOffset::new(0.0, 1.0))
            }
            (HorizontalPositionKeyword::Right, VerticalPositionKeyword::Bottom) => {
                (RelativeOffset::new(0.0, 0.0), RelativeOffset::new(1.0, 1.0))
            }
        },
    };
    let mut stops = Vec::new();
    for item in &value.items {
        match item {
            GradientItem::ColorStop(value) => {
                let position = value
                    .position
                    .as_ref()
                    .context("gradient stop without position is unsupported")?;
                let position = match position {
                    DimensionPercentage::Dimension(_) => {
                        bail!("absolute position in gradient is unsupported")
                    }
                    DimensionPercentage::Percentage(value) => value.0,
                    DimensionPercentage::Calc(_) => bail!("calc is unsupported"),
                };
                stops.push(GradientStop::new(position, convert_color(&value.color)?));
            }
            GradientItem::Hint(_) => bail!("gradient hints are not supported"),
        }
    }
    Ok(ComputedLinearGradient {
        start,
        end,
        stops,
        mode: SpreadMode::Pad,
    })
}

pub fn convert_background_color(properties: &[&Property<'static>]) -> Result<Option<Color>> {
    let bg = convert_background(properties)?;
    if let Some(bg) = bg {
        match bg {
            ComputedBackground::Solid { color } => Ok(Some(color)),
            ComputedBackground::LinearGradient(_) => {
                bail!("only background color is supported in this context")
            }
        }
    } else {
        Ok(None)
    }
}

pub fn convert_background(properties: &[&Property<'static>]) -> Result<Option<ComputedBackground>> {
    let mut color = None;
    let mut gradient = None;
    for property in properties {
        match property {
            Property::Background(backgrounds) => {
                if backgrounds.is_empty() {
                    warn!("empty vec in Property::Background");
                    continue;
                }
                if backgrounds.len() > 1 {
                    warn!("multiple backgrounds are not supported");
                }
                let background = &backgrounds[0];
                color = Some(convert_color(&background.color)?);
                match &background.image {
                    Image::None => {}
                    Image::Url(_) => bail!("url() is not supported in background"),
                    Image::Gradient(value) => match &**value {
                        Gradient::Linear(value) => {
                            gradient = Some(convert_linear_gradient(value)?);
                        }
                        _ => bail!("unsupported gradient"),
                    },
                    Image::ImageSet(_) => bail!("ImageSet is not supported in background"),
                }
            }
            Property::BackgroundColor(value) => {
                color = Some(convert_color(value)?);
            }
            _ => {}
        }
    }
    if let Some(gradient) = gradient {
        if color.is_some() {
            warn!("background color is unused because gradient is specified");
        }
        Ok(Some(ComputedBackground::LinearGradient(gradient)))
    } else if let Some(color) = color {
        Ok(Some(ComputedBackground::Solid { color }))
    } else {
        Ok(None)
    }
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
