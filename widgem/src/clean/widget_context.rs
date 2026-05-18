use {
    crate::{
        style::{css::StyleSelector, Style},
        widget_base::CustomStyle,
    },
    strict_num::FiniteF32,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WidgetContext {
    pub(crate) is_in_window: bool,

    // TODO: only present after layout pass; enable it with a widget flag?
    //pub(crate) size: Option<Size>,
    pub(crate) is_focused: bool,
    // TODO: also add is_focus_within
    pub(crate) is_under_mouse: bool,
    pub(crate) is_effectively_enabled: bool, // self & parent enabled
    pub(crate) is_effectively_visible: bool, // self & parent visible
    pub(crate) system_style: Style,
    pub(crate) custom_style: Option<CustomStyle>,
    pub(crate) style_selector: StyleSelector,
    pub(crate) effective_scale: Option<FiniteF32>, // parent scale * (self scale OR 1)
}
