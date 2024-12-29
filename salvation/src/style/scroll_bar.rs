use {
    super::{
        computed::{ComputedBackground, ComputedBorderStyle},
        css::{convert_background, convert_border, get_border_collapse, Element},
        Style,
    },
    crate::types::{LpxSuffix, PhysicalPixels},
    anyhow::Result,
    tiny_skia::Color,
};

#[derive(Debug, Clone)]
pub struct ComputedStyle {
    // TODO: variant for disabled and maybe focus
    pub border: ComputedBorderStyle,
    pub background: Option<ComputedBackground>,
    pub border_collapse: PhysicalPixels,
}

impl ComputedStyle {
    pub fn new(style: &Style, scale: f32) -> Result<ComputedStyle> {
        let element = Element::new("scroll-bar");
        let rules = style.find_rules(|s| element.matches(s));

        // TODO: Option<Color> or compute proper color
        let border = convert_border(&rules, scale, Color::TRANSPARENT);
        let background = convert_background(&rules);

        let border_collapse = if get_border_collapse(&rules) {
            // scroll_left
            //     .variants
            //     .get(&Default::default())
            //     .expect("missing style variant")
            //     .border
            //     .width
            // TODO: grid layout should detect border width
            scale.lpx().to_physical(scale)
        } else {
            0.into()
        };
        //println!("scroll_left: {scroll_left:#?}");

        Ok(Self {
            border,
            background,
            border_collapse,
        })
    }
}
