use {
    crate::{clean::WidgetContext, style::common::ComputedElementStyle},
    std::rc::Rc,
    tracing::error,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WidgetRenderContext {
    pub(crate) widget_context: WidgetContext,
}

impl WidgetRenderContext {
    pub fn compute_style<T: ComputedElementStyle>(&self) -> Rc<T> {
        self.widget_context.system_style.get(
            &self.widget_context.style_selector,
            self.widget_context
                .effective_scale
                .map(|v| v.get())
                .unwrap_or_else(|| {
                    error!(
                        "effective_scale is not available when requesting to compute style\n\
                        make sure to add visual items to a Window
                    "
                    );
                    1.0
                }),
            self.widget_context
                .custom_style
                .as_ref()
                .map(|s| s.style_sheet()),
        )
    }
}
