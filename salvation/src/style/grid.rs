use anyhow::Result;

use crate::types::Point;

use super::{
    css::{convert_padding, convert_spacing, Element, MyPseudoClass},
    FontStyle, Style,
};

#[derive(Debug, Clone)]
pub struct ComputedStyle {
    pub preferred_padding: Point,
    pub min_padding: Point,
    pub preferred_spacing: Point,
    pub min_spacing: Point,
}

impl ComputedStyle {
    pub fn new(style: &Style, scale: f32, root_font: &FontStyle) -> Result<ComputedStyle> {
        let element = Element::new("grid");
        let element_min = element.clone().with_pseudo_class(MyPseudoClass::Min);

        let properties = style.find_rules(|s| element.matches(s));
        let preferred_padding = convert_padding(&properties, scale, root_font.font_size);
        let preferred_spacing = convert_spacing(&properties, scale, root_font.font_size)?;

        let min_properties = style.find_rules(|s| element_min.matches(s));
        let min_padding = convert_padding(&min_properties, scale, root_font.font_size);
        let min_spacing = convert_spacing(&min_properties, scale, root_font.font_size)?;
        Ok(Self {
            preferred_padding,
            min_padding,
            preferred_spacing,
            min_spacing,
        })
    }
}
