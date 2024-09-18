use anyhow::Result;
use tiny_skia::Color;

use crate::types::PhysicalPixels;

use super::{
    button,
    computed::{ComputedBackground, ComputedBorderStyle},
    css::{convert_background, convert_border, get_border_collapse, Element},
    FontStyle, Style,
};

#[derive(Debug, Clone)]
pub struct ComputedStyle {
    // TODO: variant for disabled and maybe focus
    pub border: ComputedBorderStyle,
    pub background: Option<ComputedBackground>,
    pub border_collapse: PhysicalPixels,

    pub scroll_left: button::ComputedStyle,
    pub scroll_right: button::ComputedStyle,
    pub scroll_up: button::ComputedStyle,
    pub scroll_down: button::ComputedStyle,
    pub scroll_grip_x: button::ComputedStyle,
    pub scroll_grip_y: button::ComputedStyle,
    pub scroll_pager: button::ComputedStyle,
}

impl ComputedStyle {
    pub fn new(style: &Style, scale: f32, root_font: &FontStyle) -> Result<ComputedStyle> {
        let element = Element::new("scroll-bar");
        let rules = style.find_rules(|s| element.matches(s));

        // TODO: Option<Color> or compute proper color
        let border = convert_border(&rules, scale, Color::TRANSPARENT)?;
        let background = convert_background(&rules)?;

        let scroll_left = button::ComputedStyle::new(style, scale, root_font, Some("scroll_left"))?;
        let border_collapse = if get_border_collapse(&rules) {
            scroll_left
                .variants
                .get(&Default::default())
                .expect("missing style variant")
                .border
                .width
        } else {
            0.into()
        };
        //println!("scroll_left: {scroll_left:#?}");

        Ok(Self {
            border,
            background,
            border_collapse,
            scroll_left,
            scroll_right: button::ComputedStyle::new(
                style,
                scale,
                root_font,
                Some("scroll_right"),
            )?,
            scroll_up: button::ComputedStyle::new(style, scale, root_font, Some("scroll_up"))?,
            scroll_down: button::ComputedStyle::new(style, scale, root_font, Some("scroll_down"))?,
            scroll_grip_x: button::ComputedStyle::new(
                style,
                scale,
                root_font,
                Some("scroll_grip_x"),
            )?,
            scroll_grip_y: button::ComputedStyle::new(
                style,
                scale,
                root_font,
                Some("scroll_grip_y"),
            )?,
            scroll_pager: button::ComputedStyle::new(
                style,
                scale,
                root_font,
                Some("scroll_pager"),
            )?,
        })
    }
}
