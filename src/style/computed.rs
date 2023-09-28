use std::collections::HashMap;

use tiny_skia::Color;

use super::{ClassRules, ElementState, PhysicalPixels, Style, VariantStyle};

pub mod text_input {
    use tiny_skia::Color;

    use crate::{
        style::{Background, Style, TextInputVariantStyle},
        types::Point,
    };

    use super::{ComputedBorderStyle, ComputedStyleVariants};

    #[derive(Debug, Clone)]
    pub struct ComputedStyle {
        pub min_padding: Point,
        pub preferred_padding: Point,
        pub min_aspect_ratio: f32,
        pub preferred_aspect_ratio: f32,
        pub font_metrics: cosmic_text::Metrics,
        pub variants: ComputedStyleVariants<TextInputVariantStyle>,
    }

    #[derive(Debug, Clone)]
    pub struct ComputedVariantStyle {
        pub border: Option<ComputedBorderStyle>,
        #[allow(dead_code)] // TODO: implement
        pub background: Option<Background>,
        pub text_color: Color,
        pub selected_text_color: Color,
        pub selected_text_background: Color,
    }

    pub fn compute_style(style: &Style, scale: f32) -> ComputedStyle {
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

pub mod button {
    use tiny_skia::Color;

    use crate::style::Background;

    use super::ComputedBorderStyle;

    #[derive(Debug, Clone)]
    pub struct ComputedVariantStyle {
        pub border: Option<ComputedBorderStyle>,
        #[allow(dead_code)] // TODO: implement
        pub background: Option<Background>,
        pub text_color: Color,
    }
}

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
    pub width: PhysicalPixels,
    pub radius: PhysicalPixels,
}

#[derive(Debug, Clone)]
pub struct ComputedStyle {
    pub style: Style,
    pub scale: f32,

    pub font_metrics: cosmic_text::Metrics,
    pub text_input: text_input::ComputedStyle,
}

impl ComputedStyle {
    pub fn new(style: Style, scale: f32) -> Self {
        Self {
            font_metrics: style.font.to_metrics(scale),
            text_input: text_input::compute_style(&style, scale),
            style,
            scale,
        }
    }
}
