use itertools::Itertools;
use tiny_skia::Color;

use super::{
    computed::ComputedBorderStyle, condition::ClassRules, Background, BorderStyle, ElementState,
    FontStyle, Padding, Style, VariantStyle,
};

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
    type Computed = ComputedVariantStyle;

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
        ComputedVariantStyle {
            border: self.border.to_physical(scale),
            background: self.background.clone(),
            text_color: self.text_color.unwrap_or(style.palette.foreground),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ComputedVariantStyle {
    pub border: Option<ComputedBorderStyle>,
    #[allow(dead_code)] // TODO: implement
    pub background: Option<Background>,
    pub text_color: Color,
}
