use super::OldStyle;

pub fn default_style() -> OldStyle {
    json5::from_str(include_str!("../../themes/default/theme.json5")).unwrap()
}
