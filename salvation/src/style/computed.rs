use std::{any::Any, cell::RefCell, collections::HashMap, rc::Rc};

use anyhow::Result;
use log::warn;
use tiny_skia::{Color, GradientStop, SpreadMode};

use crate::types::{PhysicalPixels, Point};

use super::{
    button,
    css::{convert_background_color, convert_font, is_root, Element},
    grid, image, scroll_bar, text_input, RelativeOffset, Style,
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

#[derive(Debug)]
pub struct ComputedStyleInner {
    pub style: Rc<Style>,
    pub scale: f32,

    pub background: Color,
    pub grid: grid::ComputedStyle,
    pub font_metrics: salvation_cosmic_text::Metrics,
    pub text_input: text_input::ComputedStyle,
    pub button: button::ComputedStyle,
    pub scroll_bar: scroll_bar::ComputedStyle,
    pub image: image::ComputedStyle,

    common_cache: RefCell<HashMap<Element, Rc<CommonComputedStyle>>>,
    specific_cache: RefCell<HashMap<Element, Box<dyn Any>>>,
}

#[derive(Debug, Clone)]
pub struct ComputedStyle(pub Rc<ComputedStyleInner>);

#[derive(Debug)]
pub struct CommonComputedStyle {
    pub min_padding_with_border: Point,
    pub preferred_padding_with_border: Point,
    pub border: ComputedBorderStyle,
    pub background: Option<ComputedBackground>,
    pub text_color: tiny_skia::Color,
    pub font_metrics: salvation_cosmic_text::Metrics,
}

impl CommonComputedStyle {
    pub fn new(style: &ComputedStyle) -> Self {
        todo!()
    }
}

impl ComputedStyle {
    pub fn new(style: Rc<Style>, scale: f32) -> Result<Self> {
        let root_properties = style.find_rules(is_root);
        let background = convert_background_color(&root_properties)?;
        let font = convert_font(&root_properties, None)?;

        Ok(Self(Rc::new(ComputedStyleInner {
            scale,
            background: background.unwrap_or_else(|| {
                warn!("missing root background color");
                Color::WHITE
            }),
            font_metrics: font.to_metrics(scale),
            grid: grid::ComputedStyle::new(&style, scale, &font)?,
            text_input: text_input::ComputedStyle::new(&style, scale, &font)?,
            button: button::ComputedStyle::new(&style, scale, &font, None)?,
            scroll_bar: scroll_bar::ComputedStyle::new(&style, scale, &font)?,
            image: image::ComputedStyle::new(&style, scale)?,
            style,
            common_cache: RefCell::new(HashMap::new()),
            specific_cache: RefCell::new(HashMap::new()),
        })))
    }

    pub fn get<T: ComputedElementStyle>(&self, element: &Element) -> Rc<T> {
        let mut cache = self.0.specific_cache.borrow_mut();
        if let Some(data) = cache.get(element) {
            return data
                .downcast_ref::<Rc<T>>()
                .expect("specific style type mismatch")
                .clone();
        }
        let specific = Rc::new(T::new(self, element));
        let specific_clone = specific.clone();
        cache.insert(element.clone(), Box::new(specific));
        specific_clone
    }

    pub fn get_common(&self, element: &Element) -> Rc<CommonComputedStyle> {
        let mut cache = self.0.common_cache.borrow_mut();
        if let Some(data) = cache.get(element) {
            return data.clone();
        }
        let common = Rc::new(CommonComputedStyle::new(self));
        let common_clone = common.clone();
        cache.insert(element.clone(), common);
        common_clone
    }
}

pub trait ComputedElementStyle: Any + Sized {
    fn new(style: &ComputedStyle, element: &Element) -> Self;
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
