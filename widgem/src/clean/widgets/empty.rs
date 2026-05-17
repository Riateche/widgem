use {
    crate::clean::{BoxWidget, Widget, WidgetRenderContext},
    widgem_macros::AttributeSetters,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, AttributeSetters)]
pub struct Empty {
    #[widgem_attr(private)]
    _private: (),
}

impl Widget for Empty {
    fn render(&self, _ctx: WidgetRenderContext) -> BoxWidget {
        panic!("cannot render elementary widget (Empty)")
    }
}
