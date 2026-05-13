use {
    crate::clean::{
        render::WidgetRenderContext,
        widgets::base::{WidgetBase, WidgetBaseExt},
        BoxWidget, Widget,
    },
    widgem_macros::AttributeSetters,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, AttributeSetters)]
pub struct Text {
    #[widgem_attr(extend = WidgetBaseExt)]
    base: WidgetBase,
    #[widgem_attr(constructor)]
    text: String,
    //...
}

impl Widget for Text {
    fn render(&self, _ctx: WidgetRenderContext) -> BoxWidget {
        todo!()
    }
}
