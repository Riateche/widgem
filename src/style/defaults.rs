use super::Style;

pub fn default_style() -> Style {
    json5::from_str(include_str!("../../themes/default/theme.json5")).unwrap()
}
