use derive_more::{From, Into};
use tiny_skia::Color;

pub mod computed;
pub mod defaults;

use crate::types::Point;

use self::computed::ComputedBorderStyle;

#[derive(Debug)]
pub struct Palette {
    pub foreground: Color,
    pub background: Color,
    pub selected_text_color: Color,
    pub selected_text_background: Color,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PseudoClass {
    Disabled,
    Focused,
    MouseOver,
}

// just A && B && C for now
#[derive(Debug, Clone)]
pub struct PseudoClassCondition(Vec<PseudoClass>);

impl PseudoClassCondition {
    pub fn eval(&self, classes: &[PseudoClass]) -> bool {
        self.0.iter().all(|c| classes.contains(c))
    }
}

#[derive(Debug, Clone)]
pub struct PseudoClassRules<T>(Vec<(PseudoClassCondition, T)>);

impl<T> PseudoClassRules<T> {
    pub fn has_class(&self, class: PseudoClass) -> bool {
        self.0.iter().any(|(rule, _)| rule.0.contains(&class))
    }
    pub fn filter<'a: 'b, 'b>(
        &'a self,
        classes: &'b [PseudoClass],
    ) -> impl Iterator<Item = &'a T> + 'b {
        self.0
            .iter()
            .filter(|(c, _)| c.eval(classes))
            .map(|(_, v)| v)
    }
}

#[derive(Debug, Clone)]
pub struct TextInputStyle {
    pub min_padding: Padding,
    pub preferred_padding: Padding,
    pub min_aspect_ratio: f32,
    pub preferred_aspect_ratio: f32,
    pub font: FontStyle,
    pub variants: PseudoClassRules<TextInputVariantStyle>,
}

#[derive(Debug, Clone, Default)]
pub struct TextInputVariantStyle {
    pub border: BorderStyle,
    pub background: Option<Background>,
    pub text_color: Option<Color>,
    pub selected_text_color: Option<Color>,
    pub selected_text_background: Option<Color>,
}

impl TextInputVariantStyle {
    pub fn apply(&mut self, other: &Self) {
        self.border.apply(&other.border);
        if let Some(background) = &other.background {
            self.background = Some(background.clone());
        }
        if let Some(text_color) = other.text_color {
            self.text_color = Some(text_color);
        }
        if let Some(selected_text_color) = other.selected_text_color {
            self.selected_text_color = Some(selected_text_color);
        }
        if let Some(selected_text_background) = other.selected_text_background {
            self.selected_text_background = Some(selected_text_background);
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ButtonStyle {
    pub min_padding: Option<Padding>,
    pub preferred_padding: Option<Padding>,
    pub font: FontStyle,
    pub border: BorderStyle,
    pub background: Option<Background>,
    pub text_color: Option<Color>,
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

#[derive(Debug)]
pub struct Style {
    pub font: RootFontStyle,
    pub palette: Palette,
    pub text_input: TextInputStyle,
    pub button: PseudoClassRules<ButtonStyle>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, From, Into, Default)]
pub struct LogicalPixels(i32);

impl LogicalPixels {
    pub fn get(self) -> i32 {
        self.0
    }

    pub fn to_physical(self, scale: f32) -> PhysicalPixels {
        ((self.0 as f32 * scale).round() as i32).ppx()
    }
}

pub trait LpxSuffix {
    fn lpx(self) -> LogicalPixels;
}

impl LpxSuffix for i32 {
    fn lpx(self) -> LogicalPixels {
        LogicalPixels(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, From, Into)]
pub struct PhysicalPixels(i32);

impl PhysicalPixels {
    pub fn get(self) -> i32 {
        self.0
    }
}

pub trait PpxSuffix {
    fn ppx(self) -> PhysicalPixels;
}

impl PpxSuffix for i32 {
    fn ppx(self) -> PhysicalPixels {
        PhysicalPixels(self)
    }
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

pub fn default_style() -> Style {
    Style {
        font: RootFontStyle {
            font_size: 13.lpx(),
            line_height: 18.lpx(),
        },
        text_input: TextInputStyle {
            min_padding: Padding::new(1.lpx(), 0.lpx()),
            preferred_padding: Padding::new(5.lpx(), 4.lpx()),
            min_aspect_ratio: 2.0,
            preferred_aspect_ratio: 10.0,
            font: Default::default(),
            variants: PseudoClassRules(vec![
                (
                    PseudoClassCondition(vec![]),
                    TextInputVariantStyle {
                        border: BorderStyle {
                            color: Some(Color::from_rgba8(200, 200, 200, 255)),
                            width: Some(1.lpx()),
                            radius: Some(2.lpx()),
                        },
                        ..Default::default()
                    },
                ),
                (
                    PseudoClassCondition(vec![PseudoClass::Focused]),
                    TextInputVariantStyle {
                        border: BorderStyle {
                            color: Some(Color::from_rgba8(100, 100, 255, 255)),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                ),
                /*(
                    PseudoClassCondition(vec![PseudoClass::MouseOver]),
                    TextInputVariantStyle {
                        border: BorderStyle {
                            color: Some(Color::from_rgba8(255, 0, 0, 255)),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                ),
                (
                    PseudoClassCondition(vec![PseudoClass::Focused, PseudoClass::MouseOver]),
                    TextInputVariantStyle {
                        border: BorderStyle {
                            color: Some(Color::from_rgba8(0, 255, 0, 255)),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                ),*/
            ]),
        },
        palette: Palette {
            foreground: Color::BLACK,
            background: Color::WHITE,
            selected_text_color: Color::from_rgba8(255, 255, 255, 255),
            selected_text_background: Color::from_rgba8(48, 140, 198, 255),
        },
        button: PseudoClassRules(vec![
            (
                PseudoClassCondition(vec![]),
                ButtonStyle {
                    min_padding: Some(Padding::new(1.lpx(), 0.lpx())),
                    preferred_padding: Some(Padding::new(5.lpx(), 5.lpx())),
                    border: BorderStyle {
                        color: Some(Color::from_rgba8(171, 171, 171, 255)),
                        width: Some(1.lpx()),
                        radius: Some(2.lpx()),
                    },
                    background: Some(Background::LinearGradient(())),
                    ..Default::default()
                },
            ),
            (
                PseudoClassCondition(vec![PseudoClass::Focused]),
                ButtonStyle {
                    border: BorderStyle {
                        color: Some(Color::from_rgba8(38, 112, 158, 255)),
                        ..Default::default()
                    },
                    background: Some(Background::LinearGradient(())),
                    ..Default::default()
                },
            ),
        ]),
    }
}
