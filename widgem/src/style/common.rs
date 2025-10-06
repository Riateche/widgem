use {
    super::{
        css::{
            convert_background, convert_border, convert_font, convert_main_color, convert_padding,
            convert_zoom, is_root, PseudoClass, StyleSelector,
        },
        RelativeOffset,
    },
    crate::{
        layout::{GridAxisOptions, GridOptions},
        style::{
            css::{
                convert_layout_ignores_border, convert_spacing, get_border_collapse,
                get_text_alignment, get_vertical_alignment, is_root_min,
            },
            defaults, Styles,
        },
        types::{LpxSuffix, PhysicalPixels, Point, PpxSuffix},
    },
    std::any::Any,
    tiny_skia::{Color, GradientStop, SpreadMode},
    tracing::warn,
};

#[derive(Debug, Clone)]
pub struct ComputedBorderStyle {
    pub width: PhysicalPixels,
    pub color: Color,
    pub radius: PhysicalPixels,
}

impl Default for ComputedBorderStyle {
    fn default() -> Self {
        Self {
            width: Default::default(),
            color: Color::TRANSPARENT,
            radius: Default::default(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct BaseComputedStyle {
    pub(crate) border: ComputedBorderStyle,
    pub(crate) background: Option<ComputedBackground>,
    pub(crate) font_metrics: cosmic_text::Metrics,
    pub(crate) grid: GridOptions,
}

impl ComputedElementStyle for BaseComputedStyle {
    fn new(style: &Styles, element: &StyleSelector, scale: f32) -> Self {
        let rules = style.find_rules(|s| element.matches(s));

        let mut rules_with_root = style.find_rules(is_root);
        rules_with_root.extend(rules.clone());
        let element_min = element
            .clone()
            .with_pseudo_class(PseudoClass::Custom("min".into()));
        let mut min_rules_with_root = style.find_rules(is_root_min);
        min_rules_with_root.extend(style.find_rules(|s| element_min.matches(s)));
        let properties_with_root =
            style.find_rules(|selector| is_root(selector) || element.matches(selector));

        let scale = scale * convert_zoom(&rules);
        let font = convert_font(&rules, Some(&style.root_font_style()));
        let min_padding = convert_padding(&min_rules_with_root, scale, font.font_size);
        let preferred_padding = convert_padding(&rules_with_root, scale, font.font_size);

        let min_spacing = convert_spacing(&min_rules_with_root, scale, font.font_size);
        let preferred_spacing = convert_spacing(&rules_with_root, scale, font.font_size);

        let text_color = convert_main_color(&properties_with_root).unwrap_or_else(|| {
            warn!("text color is not specified");
            defaults::text_color()
        });
        let border = convert_border(&rules_with_root, scale, text_color);
        let background = convert_background(&rules);
        let border_collapse = if get_border_collapse(&rules_with_root) {
            // TODO: somehow fetch border width of children and use it
            1.0.lpx().to_physical(scale)
        } else {
            0.ppx()
        };

        let layout_ignores_border = convert_layout_ignores_border(&rules_with_root);
        let min_padding_with_border = if layout_ignores_border {
            min_padding
        } else {
            min_padding + Point::new(border.width, border.width)
        };
        let preferred_padding_with_border = if layout_ignores_border {
            preferred_padding
        } else {
            preferred_padding + Point::new(border.width, border.width)
        };

        let grid = GridOptions {
            x: GridAxisOptions {
                min_padding: min_padding_with_border.x(),
                min_spacing: min_spacing.x(),
                preferred_padding: preferred_padding_with_border.x(),
                preferred_spacing: preferred_spacing.x(),
                border_collapse,
                alignment: get_text_alignment(&rules_with_root),
            },
            y: GridAxisOptions {
                min_padding: min_padding_with_border.y(),
                min_spacing: min_spacing.y(),
                preferred_padding: preferred_padding_with_border.y(),
                preferred_spacing: preferred_spacing.y(),
                border_collapse,
                alignment: get_vertical_alignment(&rules_with_root),
            },
        };

        Self {
            font_metrics: font.to_metrics(scale),
            border,
            background,
            grid,
        }
    }
}

pub trait ComputedElementStyle: Any + Sized {
    fn new(styles: &Styles, element: &StyleSelector, scale: f32) -> Self;
}

#[derive(Debug, Clone)]
pub enum ComputedBackground {
    Solid { color: Color },
    LinearGradient(ComputedLinearGradient),
}

#[derive(Debug, Clone)]
pub struct ComputedLinearGradient {
    pub start: RelativeOffset,
    pub end: RelativeOffset,
    pub stops: Vec<GradientStop>,
    pub mode: SpreadMode,
}
