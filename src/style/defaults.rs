use tiny_skia::Color;

use crate::types::LpxSuffix;

use super::{
    button::{ButtonClass, ButtonVariantStyle},
    condition::{ClassCondition, ClassRules},
    text_input::{TextInputClass, TextInputVariantStyle},
    Background, BorderStyle, ButtonStyle, Padding, Palette, RootFontStyle, Style, TextInputStyle,
};

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
