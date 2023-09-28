use std::{fmt::Debug, ops::Not};

use derive_more::{From, Into};
use itertools::Itertools;
use std::hash::Hash;
use tiny_skia::Color;

pub mod computed;
pub mod defaults;

use crate::types::Point;

use self::computed::{button, text_input, ComputedBorderStyle};

#[derive(Debug, Clone)]
pub struct Palette {
    pub foreground: Color,
    pub background: Color,
    pub selected_text_color: Color,
    pub selected_text_background: Color,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextInputClass {
    Enabled,
    Focused,
    MouseOver,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextInputState {
    Enabled { focused: bool, mouse_over: bool },
    Disabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ButtonClass {
    Enabled,
    Focused,
    MouseOver,
    Pressed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ButtonState {
    Enabled {
        focused: bool,
        mouse_over: bool,
        pressed: bool,
    },
    Disabled,
}

pub trait ElementState: Eq + Hash + Sized {
    type Class: Debug + Clone; // TODO: remove Debug
    fn all() -> Vec<Self>;
    fn matches(&self, class: &Self::Class) -> bool;
}

impl ElementState for TextInputState {
    type Class = TextInputClass;

    fn all() -> Vec<Self> {
        let all_bools = [false, true];
        let mut r = all_bools
            .into_iter()
            .cartesian_product(all_bools)
            .map(|(focused, mouse_over)| Self::Enabled {
                focused,
                mouse_over,
            })
            .collect_vec();
        r.push(Self::Disabled);
        r
    }

    fn matches(&self, class: &Self::Class) -> bool {
        match class {
            TextInputClass::Enabled => matches!(self, Self::Enabled { .. }),
            TextInputClass::Focused => match self {
                Self::Enabled { focused, .. } => *focused,
                Self::Disabled => false,
            },
            TextInputClass::MouseOver => match self {
                Self::Enabled { mouse_over, .. } => *mouse_over,
                Self::Disabled => false,
            },
        }
    }
}

impl ElementState for ButtonState {
    type Class = ButtonClass;

    fn all() -> Vec<Self> {
        let all_bools = [false, true];
        let mut r = all_bools
            .into_iter()
            .cartesian_product(all_bools)
            .cartesian_product(all_bools)
            .map(|((focused, mouse_over), pressed)| Self::Enabled {
                focused,
                mouse_over,
                pressed,
            })
            .collect_vec();
        r.push(Self::Disabled);
        r
    }

    fn matches(&self, class: &Self::Class) -> bool {
        match class {
            ButtonClass::Enabled => matches!(self, Self::Enabled { .. }),
            ButtonClass::Focused => match self {
                Self::Enabled { focused, .. } => *focused,
                Self::Disabled => false,
            },
            ButtonClass::MouseOver => match self {
                Self::Enabled { mouse_over, .. } => *mouse_over,
                Self::Disabled => false,
            },
            ButtonClass::Pressed => match self {
                Self::Enabled { pressed, .. } => *pressed,
                Self::Disabled => false,
            },
        }
    }
}

fn eval_condition<V>(variant: &V, condition: &ClassCondition<V::Class>) -> bool
where
    V: ElementState,
{
    match condition {
        ClassCondition::Has(class) => variant.matches(class),
        ClassCondition::Not(condition) => !eval_condition(variant, condition),
        ClassCondition::And(conditions) => conditions.iter().all(|c| eval_condition(variant, c)),
        ClassCondition::Or(conditions) => conditions.iter().any(|c| eval_condition(variant, c)),
    }
}

// just A && B && C for now
#[derive(Debug, Clone)]
pub enum ClassCondition<Class> {
    Has(Class),
    Not(Box<Self>),
    And(Vec<Self>),
    Or(Vec<Self>),
}

impl<Class> From<Class> for ClassCondition<Class> {
    fn from(value: Class) -> Self {
        ClassCondition::Has(value)
    }
}

impl<Class> ClassCondition<Class> {
    pub fn has(class: Class) -> Self {
        Self::Has(class)
    }

    pub fn not(condition: impl Into<Self>) -> Self {
        Self::Not(Box::new(condition.into()))
    }

    pub fn depends_on(&self, class: &Class) -> bool
    where
        Class: PartialEq,
    {
        match self {
            ClassCondition::Has(c) => c == class,
            ClassCondition::Not(c) => c.depends_on(class),
            ClassCondition::And(c) | ClassCondition::Or(c) => c.iter().any(|c| c.depends_on(class)),
        }
    }

    pub fn and(&self, condition: impl Into<Self>) -> Self
    where
        Class: Clone,
    {
        // TODO: simplify multiple ands into one
        Self::And(vec![self.clone(), condition.into()])
    }

    pub fn or(&self, condition: impl Into<Self>) -> Self
    where
        Class: Clone,
    {
        // TODO: simplify multiple ands into one
        Self::Or(vec![self.clone(), condition.into()])
    }

    pub fn always() -> Self {
        Self::And(Vec::new())
    }
}

impl<Class> Not for ClassCondition<Class> {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self::Not(Box::new(self))
    }
}

#[derive(Debug, Clone)]
pub struct ClassRules<T: VariantStyle>(
    pub Vec<(ClassCondition<<T::State as ElementState>::Class>, T)>,
);

impl<T: VariantStyle> ClassRules<T> {
    pub fn depends_on(&self, class: <T::State as ElementState>::Class) -> bool
    where
        <T::State as ElementState>::Class: PartialEq,
    {
        self.0.iter().any(|(rule, _)| rule.depends_on(&class))
    }

    pub fn get(&self, variant: &T::State) -> T {
        let mut r = T::default();
        for (condition, item) in &self.0 {
            if eval_condition(variant, condition) {
                r.apply(item);
            }
        }
        r
    }
}

#[derive(Debug, Clone)]
pub struct TextInputStyle {
    pub min_padding: Padding,
    pub preferred_padding: Padding,
    pub min_aspect_ratio: f32,
    pub preferred_aspect_ratio: f32,
    pub font: FontStyle,
    pub variants: ClassRules<TextInputVariantStyle>,
}

#[derive(Debug, Clone, Default)]
pub struct TextInputVariantStyle {
    pub border: BorderStyle,
    pub background: Option<Background>,
    pub text_color: Option<Color>,
    pub selected_text_color: Option<Color>,
    pub selected_text_background: Option<Color>,
}

pub trait VariantStyle: Default {
    type State: ElementState;
    type Computed;
    fn apply(&mut self, other: &Self);
    fn compute(&self, style: &Style, scale: f32) -> Self::Computed;
}

impl VariantStyle for TextInputVariantStyle {
    type State = TextInputState;
    type Computed = text_input::ComputedVariantStyle;

    fn apply(&mut self, other: &Self) {
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

    fn compute(&self, style: &Style, scale: f32) -> Self::Computed {
        // TODO: get more default properties from style root?
        // TODO: default border from style root
        text_input::ComputedVariantStyle {
            border: self.border.to_physical(scale),
            background: self.background.clone(),
            text_color: self.text_color.unwrap_or(style.palette.foreground),
            selected_text_color: self
                .selected_text_color
                .unwrap_or(style.palette.selected_text_color),
            selected_text_background: self
                .selected_text_background
                .unwrap_or(style.palette.selected_text_background),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ButtonStyle {
    pub min_padding: Padding,
    pub preferred_padding: Padding,
    pub font: FontStyle,
    pub variants: ClassRules<ButtonVariantStyle>,
}

#[derive(Debug, Clone, Default)]
pub struct ButtonVariantStyle {
    pub border: BorderStyle,
    pub background: Option<Background>,
    pub text_color: Option<Color>,
}

impl VariantStyle for ButtonVariantStyle {
    type State = ButtonState;
    type Computed = button::ComputedVariantStyle;

    fn apply(&mut self, other: &Self) {
        self.border.apply(&other.border);
        if let Some(background) = &other.background {
            self.background = Some(background.clone());
        }
        if let Some(text_color) = other.text_color {
            self.text_color = Some(text_color);
        }
    }

    fn compute(&self, style: &Style, scale: f32) -> Self::Computed {
        // TODO: get more default properties from style root?
        // TODO: default border from style root
        button::ComputedVariantStyle {
            border: self.border.to_physical(scale),
            background: self.background.clone(),
            text_color: self.text_color.unwrap_or(style.palette.foreground),
        }
    }
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
            variants: ClassRules(vec![
                (
                    ClassCondition::always(),
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
                    ClassCondition::has(TextInputClass::Focused),
                    TextInputVariantStyle {
                        border: BorderStyle {
                            color: Some(Color::from_rgba8(100, 100, 255, 255)),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                ),
                (
                    ClassCondition::has(TextInputClass::MouseOver),
                    TextInputVariantStyle {
                        border: BorderStyle {
                            color: Some(Color::from_rgba8(255, 0, 0, 255)),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                ),
                (
                    ClassCondition::has(TextInputClass::Focused).and(TextInputClass::MouseOver),
                    TextInputVariantStyle {
                        border: BorderStyle {
                            color: Some(Color::from_rgba8(0, 255, 0, 255)),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                ),
            ]),
        },
        palette: Palette {
            foreground: Color::BLACK,
            background: Color::WHITE,
            selected_text_color: Color::from_rgba8(255, 255, 255, 255),
            selected_text_background: Color::from_rgba8(48, 140, 198, 255),
        },
        button: ButtonStyle {
            min_padding: Padding::new(1.lpx(), 0.lpx()),
            preferred_padding: Padding::new(5.lpx(), 5.lpx()),
            font: Default::default(),
            variants: ClassRules(vec![
                (
                    ClassCondition::always(),
                    ButtonVariantStyle {
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
                    ClassCondition::has(ButtonClass::Focused),
                    ButtonVariantStyle {
                        border: BorderStyle {
                            color: Some(Color::from_rgba8(38, 112, 158, 255)),
                            ..Default::default()
                        },
                        background: Some(Background::LinearGradient(())),
                        ..Default::default()
                    },
                ),
            ]),
        },
    }
}
