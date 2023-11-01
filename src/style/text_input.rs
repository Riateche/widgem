use std::collections::HashMap;

use anyhow::Result;
use itertools::Itertools;
use lightningcss::selector::{PseudoClass, Selector};
use log::warn;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};

use crate::{
    style::{
        computed::{
            convert_background, convert_background_color, convert_border, convert_main_color,
        },
        css::{is_root, is_selection},
        defaults,
    },
    types::{PhysicalPixels, Point},
};

use super::{
    computed::{
        convert_font, convert_padding, convert_width, ComputedBackground, ComputedBorderStyle,
    },
    css::{as_tag_with_class, is_tag_with_custom_class, is_tag_with_no_class},
    defaults::{DEFAULT_MIN_WIDTH_EM, DEFAULT_PREFERRED_WIDTH_EM},
    ElementState, RootFontStyle, Style,
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

impl Default for TextInputState {
    fn default() -> Self {
        Self::Enabled {
            focused: false,
            mouse_over: false,
        }
    }
}

impl TextInputState {
    pub fn matches(&self, selector: &Selector) -> bool {
        if let Some(data) = as_tag_with_class(selector) {
            if data.tag != "text-input" {
                return false;
            }
            if let Some(class) = data.class {
                match class {
                    PseudoClass::Hover => {
                        if let TextInputState::Enabled { mouse_over, .. } = self {
                            *mouse_over
                        } else {
                            false
                        }
                    }
                    PseudoClass::Focus => {
                        if let TextInputState::Enabled { focused, .. } = self {
                            *focused
                        } else {
                            false
                        }
                    }
                    PseudoClass::Disabled => match self {
                        TextInputState::Enabled { .. } => false,
                        TextInputState::Disabled => true,
                    },
                    PseudoClass::Enabled => match self {
                        TextInputState::Enabled { .. } => true,
                        TextInputState::Disabled => false,
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

impl ElementState for TextInputState {
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
}

#[derive(Debug, Clone)]
pub struct ComputedStyle {
    pub min_padding_with_border: Point,
    pub preferred_padding_with_border: Point,
    pub min_width: PhysicalPixels,
    pub preferred_width: PhysicalPixels,
    pub font_metrics: cosmic_text::Metrics,
    pub variants: HashMap<TextInputState, ComputedVariantStyle>,
}

impl ComputedStyle {
    pub fn new(style: &Style, scale: f32, root_font: &RootFontStyle) -> Result<ComputedStyle> {
        let properties = style.find_rules(|s| is_tag_with_no_class(s, "text-input"));
        let font = convert_font(&properties, Some(root_font))?;
        let preferred_padding = convert_padding(&properties, scale, font.font_size)?;
        let preferred_width = convert_width(&properties, scale, font.font_size)?
            .unwrap_or_else(|| (font.font_size * DEFAULT_PREFERRED_WIDTH_EM).to_physical(scale));

        let min_properties = style.find_rules(|s| {
            is_tag_with_no_class(s, "text-input")
                || is_tag_with_custom_class(s, "text-input", "min")
        });
        let min_padding = convert_padding(&min_properties, scale, font.font_size)?;
        let min_width = convert_width(&min_properties, scale, font.font_size)?
            .unwrap_or_else(|| (font.font_size * DEFAULT_MIN_WIDTH_EM).to_physical(scale));

        // TODO: variant-specific selection css rules?
        let selection_properties = style.find_rules(is_selection);
        let selected_text_color = convert_main_color(&selection_properties)?.unwrap_or_else(|| {
            warn!("selected text color is unspecified");
            defaults::selected_text_color()
        });
        let selected_text_background = convert_background_color(&selection_properties)?
            .unwrap_or_else(|| {
                warn!("selected text background is unspecified");
                defaults::selected_text_background()
            });

        let variants = TextInputState::all()
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
                    selected_text_color,
                    selected_text_background,
                };
                anyhow::Ok((state, style))
            })
            .collect::<anyhow::Result<HashMap<_, _>>>()?;

        let border_width = variants
            .get(&TextInputState::default())
            .expect("expected item for each state")
            .border
            .width;

        Ok(Self {
            min_padding_with_border: min_padding
                + Point::new(border_width.get(), border_width.get()),
            preferred_padding_with_border: preferred_padding
                + Point::new(border_width.get(), border_width.get()),
            min_width,
            preferred_width,
            font_metrics: font.to_metrics(scale),
            variants,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ComputedVariantStyle {
    pub border: ComputedBorderStyle,
    #[allow(dead_code)] // TODO: implement
    pub background: Option<ComputedBackground>,
    pub text_color: tiny_skia::Color,
    pub selected_text_color: tiny_skia::Color,
    pub selected_text_background: tiny_skia::Color,
}
