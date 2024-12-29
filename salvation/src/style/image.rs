use {
    super::{
        computed::ComputedBorderStyle,
        css::{convert_border, convert_zoom, Element},
        defaults, Style,
    },
    anyhow::Result,
};

#[derive(Debug, Clone)]
pub struct ComputedStyle {
    pub border: ComputedBorderStyle,
    pub scale: f32,
}

impl ComputedStyle {
    pub fn new(style: &Style, mut scale: f32) -> Result<ComputedStyle> {
        let element = Element::new("image");
        let properties = style.find_rules(|s| element.matches(s));
        scale *= convert_zoom(&properties);
        // TODO: use main color
        let border = convert_border(&properties, scale, defaults::text_color());
        Ok(Self { border, scale })
    }
}
