use tiny_skia::Color;

use crate::types::LpxSuffix;

use super::{
    button::{ButtonClass, ButtonVariantStyle},
    condition::{ClassCondition, ClassRules},
    text_input::{TextInputClass, TextInputVariantStyle},
    Background, BorderStyle, ButtonStyle, GradientStop, LinearGradient, Padding, Palette,
    RelativeOffset, RootFontStyle, SpreadMode, Style, TextInputStyle,
};

pub fn default_style() -> Style {
    let button_gradient = LinearGradient {
        start: RelativeOffset { x: 0.0, y: 0.0 },
        end: RelativeOffset { x: 0.0, y: 1.0 },
        stops: vec![
            GradientStop::new(0.0, Color::from_rgba8(254, 254, 254, 255).into()),
            GradientStop::new(1.0, Color::from_rgba8(238, 238, 238, 255).into()),
        ],
        mode: SpreadMode::Pad,
    };

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
                            color: Some(Color::from_rgba8(200, 200, 200, 255).into()),
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
                            color: Some(Color::from_rgba8(100, 100, 255, 255).into()),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                ),
                (
                    ClassCondition::has(TextInputClass::MouseOver),
                    TextInputVariantStyle {
                        border: BorderStyle {
                            color: Some(Color::from_rgba8(255, 0, 0, 255).into()),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                ),
                (
                    ClassCondition::has(TextInputClass::Focused).and(TextInputClass::MouseOver),
                    TextInputVariantStyle {
                        border: BorderStyle {
                            color: Some(Color::from_rgba8(0, 255, 0, 255).into()),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                ),
            ]),
        },
        palette: Palette {
            foreground: Color::BLACK.into(),
            background: Color::WHITE.into(),
            selected_text_color: Color::from_rgba8(255, 255, 255, 255).into(),
            selected_text_background: Color::from_rgba8(48, 140, 198, 255).into(),
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
                            color: Some(Color::from_rgba8(196, 196, 196, 255).into()),
                            width: Some(1.lpx()),
                            radius: Some(2.lpx()),
                        },
                        background: Some(Background::LinearGradient(button_gradient.clone())),

                        ..Default::default()
                    },
                ),
                (
                    ClassCondition::not(ButtonClass::Enabled),
                    ButtonVariantStyle {
                        text_color: Some(Color::from_rgba8(191, 191, 191, 255).into()),
                        ..Default::default()
                    },
                ),
                (
                    ClassCondition::has(ButtonClass::MouseOver),
                    ButtonVariantStyle {
                        background: Some(Background::LinearGradient(LinearGradient {
                            stops: vec![
                                GradientStop::new(
                                    1.0,
                                    Color::from_rgba8(254, 254, 254, 255).into(),
                                ),
                                GradientStop::new(
                                    1.0,
                                    Color::from_rgba8(247, 247, 247, 255).into(),
                                ),
                            ],
                            ..button_gradient
                        })),
                        ..Default::default()
                    },
                ),
                (
                    ClassCondition::has(ButtonClass::Pressed),
                    ButtonVariantStyle {
                        background: Some(Background::Solid(
                            Color::from_rgba8(219, 219, 219, 255).into(),
                        )),
                        ..Default::default()
                    },
                ),
                (
                    ClassCondition::has(ButtonClass::Focused),
                    ButtonVariantStyle {
                        border: BorderStyle {
                            color: Some(Color::from_rgba8(38, 112, 158, 255).into()),
                            ..BorderStyle::default()
                        },
                        ..Default::default()
                    },
                ),
            ]),
        },
    }
}
