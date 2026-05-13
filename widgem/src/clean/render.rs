use {
    crate::{
        style::{common::ComputedElementStyle, css::StyleSelector, Style},
        types::Size,
        widget_base::CustomStyle,
    },
    std::rc::Rc,
    strict_num::FiniteF32,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WidgetRenderContext {
    // TODO: only present after layout pass; enable it with a widget flag?
    pub(crate) size: Option<Size>,
    pub(crate) is_focused: bool,
    // TODO: also add is_focus_within
    pub(crate) is_under_mouse: bool,
    pub(crate) is_effectively_enabled: bool, // self & parent enabled
    pub(crate) is_effectively_visible: bool, // self & parent visible
    pub(crate) system_style: Style,
    pub(crate) custom_style: Option<CustomStyle>,
    pub(crate) style_selector: StyleSelector,
    pub(crate) effective_scale: FiniteF32, // parent scale * (self scale OR 1)
}

impl WidgetRenderContext {
    pub fn compute_style<T: ComputedElementStyle>(&self) -> Rc<T> {
        self.system_style.get(
            &self.style_selector,
            self.effective_scale.get(),
            self.custom_style.as_ref().map(|s| s.style_sheet()),
        )
    }
}
