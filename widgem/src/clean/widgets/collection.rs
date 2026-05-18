use {
    crate::{
        clean::{BoxWidget, Widget, WidgetRenderContext},
        ChildKey,
    },
    std::collections::BTreeMap,
    widgem_macros::AttributeSetters,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, AttributeSetters)]
pub struct Collection {
    #[widgem_attr(private)]
    items: BTreeMap<ChildKey, BoxWidget>,
}

impl Collection {
    pub fn item(mut self, key: impl Into<ChildKey>, widget: BoxWidget) -> Self {
        self.items.insert(key.into(), widget);
        self
    }

    pub fn items(mut self, iter: impl IntoIterator<Item = (ChildKey, BoxWidget)>) -> Self {
        self.items.extend(iter);
        self
    }
}

impl Widget for Collection {
    fn render(&self, _ctx: WidgetRenderContext) -> BoxWidget {
        panic!("Collection is the final stage of rendering process and cannot be further rendered");
    }
}
