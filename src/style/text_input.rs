use itertools::Itertools;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};

use crate::types::Point;

use super::{
    computed::{ComputedBorderStyle, ComputedStyleVariants},
    condition::ClassRules,
    Background, BorderStyle, ColorRef, ElementState, FontStyle, Padding, Style, VariantStyle,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, EnumString, Display)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextInputStyle {
    pub min_padding: Padding,
    pub preferred_padding: Padding,
    pub min_aspect_ratio: f32,
    pub preferred_aspect_ratio: f32,
    pub font: FontStyle,
    pub variants: ClassRules<TextInputVariantStyle>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextInputVariantStyle {
    pub border: BorderStyle,
    pub background: Option<Background>,
    pub text_color: Option<ColorRef>,
    pub selected_text_color: Option<ColorRef>,
    pub selected_text_background: Option<ColorRef>,
}

impl VariantStyle for TextInputVariantStyle {
    type State = TextInputState;
    type Computed = ComputedVariantStyle;

    fn apply(&mut self, other: &Self) {
        self.border.apply(&other.border);
        if let Some(background) = &other.background {
            self.background = Some(background.clone());
        }
        if let Some(text_color) = &other.text_color {
            self.text_color = Some(text_color.clone());
        }
        if let Some(selected_text_color) = &other.selected_text_color {
            self.selected_text_color = Some(selected_text_color.clone());
        }
        if let Some(selected_text_background) = &other.selected_text_background {
            self.selected_text_background = Some(selected_text_background.clone());
        }
    }

    fn compute(&self, style: &Style, scale: f32) -> Self::Computed {
        // TODO: get more default properties from style root?
        // TODO: default border from style root
        ComputedVariantStyle {
            border: self.border.to_physical(scale, &style.palette),
            background: self.background.clone(),
            text_color: style
                .palette
                .get(self.text_color.as_ref().unwrap_or(&ColorRef::foreground)),
            selected_text_color: style.palette.get(
                self.selected_text_color
                    .as_ref()
                    .unwrap_or(&ColorRef::selected_text_color),
            ),
            selected_text_background: style.palette.get(
                self.selected_text_background
                    .as_ref()
                    .unwrap_or(&ColorRef::selected_text_background),
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ComputedStyle {
    pub min_padding: Point,
    pub preferred_padding: Point,
    pub min_aspect_ratio: f32,
    pub preferred_aspect_ratio: f32,
    pub font_metrics: cosmic_text::Metrics,
    pub variants: ComputedStyleVariants<TextInputVariantStyle>,
}

impl ComputedStyle {
    pub fn new(style: &Style, scale: f32) -> ComputedStyle {
        let mut font = style.font.clone();
        font.apply(&style.text_input.font);

        ComputedStyle {
            min_padding: style.text_input.min_padding.to_physical(scale),
            preferred_padding: style.text_input.preferred_padding.to_physical(scale),
            min_aspect_ratio: style.text_input.min_aspect_ratio,
            preferred_aspect_ratio: style.text_input.preferred_aspect_ratio,
            font_metrics: font.to_metrics(scale),
            variants: ComputedStyleVariants::new(&style.text_input.variants, style, scale),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ComputedVariantStyle {
    pub border: Option<ComputedBorderStyle>,
    #[allow(dead_code)] // TODO: implement
    pub background: Option<Background>,
    pub text_color: tiny_skia::Color,
    pub selected_text_color: tiny_skia::Color,
    pub selected_text_background: tiny_skia::Color,
}
