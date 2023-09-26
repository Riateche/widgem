use tiny_skia::Color;

use super::{PhysicalPixels, Style};

pub mod text_input {
    use tiny_skia::Color;

    use crate::{
        style::{Background, PseudoClass, Style, TextInputVariantStyle},
        types::Point,
    };

    use super::ComputedBorderStyle;

    #[derive(Debug, Clone)]
    pub struct ComputedStyle {
        pub min_padding: Point,
        pub preferred_padding: Point,
        pub min_aspect_ratio: f32,
        pub preferred_aspect_ratio: f32,
        pub font_metrics: cosmic_text::Metrics,
        pub has_mouse_over: bool,

        pub normal: ComputedVariantStyle,
        pub disabled: ComputedVariantStyle,
        pub mouse_over: ComputedVariantStyle,
        pub focused: ComputedVariantStyle,
        pub focused_mouse_over: ComputedVariantStyle,
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

    impl ComputedVariantStyle {
        fn compute(style: &Style, classes: &[PseudoClass], scale: f32) -> Self {
            let mut current = TextInputVariantStyle::default();
            for item in style.text_input.variants.filter(classes) {
                println!("item {item:?}");
                current.apply(item);
            }
            let mut font = style.font.clone();
            font.apply(&style.text_input.font);
            // TODO: get more default properties from style root?
            // TODO: default border from style root
            Self {
                border: current.border.to_physical(scale),
                background: current.background,
                text_color: current.text_color.unwrap_or(style.palette.foreground),
                selected_text_color: current
                    .selected_text_color
                    .unwrap_or(style.palette.selected_text_color),
                selected_text_background: current
                    .selected_text_background
                    .unwrap_or(style.palette.selected_text_background),
            }
        }
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
            has_mouse_over: style.text_input.variants.has_class(PseudoClass::MouseOver),

            normal: ComputedVariantStyle::compute(style, &[], scale),
            focused: ComputedVariantStyle::compute(style, &[PseudoClass::Focused], scale),
            disabled: ComputedVariantStyle::compute(style, &[PseudoClass::Disabled], scale),
            mouse_over: ComputedVariantStyle::compute(style, &[PseudoClass::MouseOver], scale),
            focused_mouse_over: ComputedVariantStyle::compute(
                style,
                &[PseudoClass::Focused, PseudoClass::MouseOver],
                scale,
            ),
        }
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
