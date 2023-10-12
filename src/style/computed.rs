use std::collections::HashMap;

use tiny_skia::{Color, GradientStop, SpreadMode};

use crate::types::PhysicalPixels;

use super::{
    button, condition::ClassRules, text_input, ElementState, RelativeOffset, Style, VariantStyle,
};

#[derive(Debug, Clone)]
pub struct ComputedStyleVariants<T: VariantStyle>(HashMap<T::State, T::Computed>);

impl<T: VariantStyle> ComputedStyleVariants<T> {
    pub fn new(rules: &ClassRules<T>, style: &Style, scale: f32) -> Self {
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
    pub style: Style,
    pub scale: f32,

    pub font_metrics: cosmic_text::Metrics,
    pub text_input: text_input::ComputedStyle,
    pub button: button::ComputedStyle,
}

impl ComputedStyle {
    pub fn new(style: Style, scale: f32) -> Self {
        Self {
            font_metrics: style.font.to_metrics(scale),
            text_input: text_input::ComputedStyle::new(&style, scale),
            button: button::ComputedStyle::new(&style, scale),
            style,
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
