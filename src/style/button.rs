use std::collections::HashMap;

use anyhow::Result;
use itertools::Itertools;
use lightningcss::selector::{PseudoClass, Selector};
use log::warn;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};

use crate::{
    style::defaults,
    types::{LogicalPixels, Point},
};

use super::{
    computed::{
        convert_background, convert_border, convert_font, convert_main_color, convert_padding,
        ComputedBackground, ComputedBorderStyle, ComputedStyleVariants,
    },
    condition::ClassRules,
    css::{as_tag_with_class, is_root, is_tag_with_custom_class, is_tag_with_no_class},
    Background, BorderStyle, ColorRef, ElementState, FontStyle, OldStyle, Padding, RootFontStyle,
    Style, VariantStyle,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, EnumString, Display)]
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

impl Default for ButtonState {
    fn default() -> Self {
        Self::Enabled {
            focused: false,
            mouse_over: false,
            pressed: false,
        }
    }
}

impl ButtonState {
    pub fn matches(&self, selector: &Selector) -> bool {
        if let Some(data) = as_tag_with_class(selector) {
            if data.tag != "button" {
                return false;
            }
            if let Some(class) = data.class {
                match class {
                    PseudoClass::Hover => {
                        if let Self::Enabled { mouse_over, .. } = self {
                            *mouse_over
                        } else {
                            false
                        }
                    }
                    PseudoClass::Focus => {
                        if let Self::Enabled { focused, .. } = self {
                            *focused
                        } else {
                            false
                        }
                    }
                    PseudoClass::Active => {
                        if let Self::Enabled { pressed, .. } = self {
                            *pressed
                        } else {
                            false
                        }
                    }
                    PseudoClass::Disabled => match self {
                        Self::Enabled { .. } => false,
                        Self::Disabled => true,
                    },
                    PseudoClass::Enabled => match self {
                        Self::Enabled { .. } => true,
                        Self::Disabled => false,
                    },
                    _ => false,
                }
            } else {
                true
            }
        } else {
            false
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButtonStyle {
    pub border_width: Option<LogicalPixels>,
    pub min_padding: Padding,
    pub preferred_padding: Padding,
    pub font: FontStyle,
    pub variants: ClassRules<ButtonVariantStyle>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ButtonVariantStyle {
    pub border: BorderStyle,
    pub background: Option<Background>,
    pub text_color: Option<ColorRef>,
}

impl VariantStyle for ButtonVariantStyle {
    type State = ButtonState;
    type Computed = ComputedVariantStyle;

    fn apply(&mut self, other: &Self) {
        self.border.apply(&other.border);
        if let Some(background) = &other.background {
            self.background = Some(background.clone());
        }
        if let Some(text_color) = &other.text_color {
            self.text_color = Some(text_color.clone());
        }
    }

    fn compute(&self, style: &OldStyle, scale: f32) -> Self::Computed {
        // TODO: get more default properties from style root?
        // TODO: default border from style root
        ComputedVariantStyle {
            border: self.border.to_physical(scale, &style.palette),
            background: self.background.as_ref().map(|b| b.compute(&style.palette)),
            text_color: style
                .palette
                .get(self.text_color.as_ref().unwrap_or(&ColorRef::foreground)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ComputedVariantStyle {
    pub border: ComputedBorderStyle,
    #[allow(dead_code)] // TODO: implement
    pub background: Option<ComputedBackground>,
    pub text_color: tiny_skia::Color,
}

#[derive(Debug, Clone)]
pub struct ComputedStyle {
    pub min_padding_with_border: Point,
    pub preferred_padding_with_border: Point,
    pub font_metrics: cosmic_text::Metrics,
    pub variants: HashMap<ButtonState, ComputedVariantStyle>,
    pub old_variants: ComputedStyleVariants<ButtonVariantStyle>,
}

impl ComputedStyle {
    pub fn new(style: &Style, scale: f32, root_font: &RootFontStyle) -> Result<ComputedStyle> {
        let properties = style.find_rules(|s| is_tag_with_no_class(s, "button"));
        let font = convert_font(&properties, Some(root_font))?;
        let preferred_padding = convert_padding(&properties, scale, font.font_size)?;

        let min_properties = style.find_rules(|s| {
            is_tag_with_no_class(s, "button") || is_tag_with_custom_class(s, "button", "min")
        });
        let min_padding = convert_padding(&min_properties, scale, font.font_size)?;

        let variants = ButtonState::all()
            .into_iter()
            .map(|state| {
                let rules = style.find_rules(|selector| state.matches(selector));
                let rules_with_root =
                    style.find_rules(|selector| is_root(selector) || state.matches(selector));
                let text_color = convert_main_color(&rules_with_root)?.unwrap_or_else(|| {
                    warn!("main text color is unspecified");
                    defaults::text_color()
                });
                let border = convert_border(&rules, scale, text_color)?;
                let background = convert_background(&rules)?;

                let style = ComputedVariantStyle {
                    border,
                    background,
                    text_color,
                };
                anyhow::Ok((state, style))
            })
            .collect::<Result<HashMap<_, _>>>()?;

        let border_width = variants
            .get(&ButtonState::default())
            .expect("expected item for each state")
            .border
            .width;

        Ok(Self {
            min_padding_with_border: min_padding
                + Point::new(border_width.get(), border_width.get()),
            preferred_padding_with_border: preferred_padding
                + Point::new(border_width.get(), border_width.get()),
            font_metrics: font.to_metrics(scale),
            variants,
            old_variants: ComputedStyleVariants(HashMap::new()),
        })
    }

    pub fn old_new(style: &OldStyle, scale: f32) -> ComputedStyle {
        let mut font = style.font.clone();
        font.apply(&style.button.font);

        ComputedStyle {
            min_padding_with_border: style.button.min_padding.to_physical(scale),
            preferred_padding_with_border: style.button.preferred_padding.to_physical(scale),
            font_metrics: font.to_metrics(scale),
            old_variants: ComputedStyleVariants::new(&style.button.variants, style, scale),
            variants: HashMap::new(),
        }
    }
}
