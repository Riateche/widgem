use {
    super::{
        css::{
            convert_background, convert_background_color, convert_border, convert_font,
            convert_main_color, convert_padding, convert_zoom, is_root, Element, PseudoClass,
        },
        grid, image, scroll_bar, text_input, FontStyle, RelativeOffset, Style,
    },
    crate::{
        style::defaults,
        types::{PhysicalPixels, Point},
    },
    anyhow::Result,
    log::warn,
    std::{any::Any, cell::RefCell, collections::HashMap, rc::Rc},
    tiny_skia::{Color, GradientStop, SpreadMode},
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
    pub font_style: FontStyle,
    pub font_metrics: cosmic_text::Metrics,
    pub text_input: text_input::ComputedStyle,
    pub scroll_bar: scroll_bar::ComputedStyle,
    pub image: image::ComputedStyle,

    common_cache: RefCell<HashMap<Element, Rc<CommonComputedStyle>>>,
    specific_cache: RefCell<HashMap<Element, Box<dyn Any>>>,
}

#[derive(Clone)]
pub struct ComputedStyle(pub Rc<ComputedStyleInner>);

impl std::fmt::Debug for ComputedStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ComputedStyle").finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub struct CommonComputedStyle {
    pub min_padding_with_border: Point,
    pub preferred_padding_with_border: Point,
    pub border: ComputedBorderStyle,
    pub background: Option<ComputedBackground>,
    pub text_color: tiny_skia::Color,
    pub font_metrics: cosmic_text::Metrics,
}

impl Default for CommonComputedStyle {
    fn default() -> Self {
        Self {
            min_padding_with_border: Default::default(),
            preferred_padding_with_border: Default::default(),
            border: Default::default(),
            background: Default::default(),
            text_color: defaults::text_color(),
            font_metrics: Default::default(),
        }
    }
}

impl CommonComputedStyle {
    pub fn new(style: &ComputedStyle, element: &Element) -> Self {
        let properties = style.0.style.find_rules(|s| element.matches(s));
        let element_min = element
            .clone()
            .with_pseudo_class(PseudoClass::Custom("min".into()));
        let min_properties = style.0.style.find_rules(|s| element_min.matches(s));
        let properties_with_root = style
            .0
            .style
            .find_rules(|selector| is_root(selector) || element.matches(selector));

        let scale = style.0.scale * convert_zoom(&properties);
        let font = convert_font(&properties, Some(&style.0.font_style));
        let preferred_padding = convert_padding(&properties, scale, font.font_size);

        let min_padding = convert_padding(&min_properties, scale, font.font_size);

        let text_color = convert_main_color(&properties_with_root).unwrap_or_else(|| {
            warn!("text color is not specified");
            defaults::text_color()
        });
        let border = convert_border(&properties, scale, text_color);
        let background = convert_background(&properties);

        Self {
            min_padding_with_border: min_padding + Point::new(border.width, border.width),
            preferred_padding_with_border: preferred_padding
                + Point::new(border.width, border.width),
            font_metrics: font.to_metrics(scale),
            border,
            background,
            text_color,
        }
    }
}

impl ComputedStyle {
    pub fn new(style: Rc<Style>, scale: f32) -> Result<Self> {
        let root_properties = style.find_rules(is_root);
        let background = convert_background_color(&root_properties)?;
        let font_style = convert_font(&root_properties, None);

        Ok(Self(Rc::new(ComputedStyleInner {
            scale,
            background: background.unwrap_or_else(|| {
                warn!("missing root background color");
                Color::WHITE
            }),
            font_metrics: font_style.to_metrics(scale),
            grid: grid::ComputedStyle::new(&style, scale, &font_style)?,
            text_input: text_input::ComputedStyle::new(&style, scale, &font_style)?,
            scroll_bar: scroll_bar::ComputedStyle::new(&style, scale)?,
            image: image::ComputedStyle::new(&style, scale)?,
            style,
            common_cache: RefCell::new(HashMap::new()),
            specific_cache: RefCell::new(HashMap::new()),
            font_style,
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
        let common = Rc::new(CommonComputedStyle::new(self, element));
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
