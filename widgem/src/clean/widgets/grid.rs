use {
    crate::{
        clean::{
            render::WidgetRenderContext,
            widgets::{WidgetBase, WidgetBaseExt},
            BoxWidget, Widget,
        },
        layout::LayoutItemOptions,
        ChildKey,
    },
    widgem_macros::AttributeSetters,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct GridItem {
    key: ChildKey,
    // TODO: rename to GridItemOptions
    options: LayoutItemOptions,
    widget: BoxWidget,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, AttributeSetters)]
pub struct Grid {
    #[widgem_attr(extend = WidgetBaseExt)]
    base: WidgetBase,
    #[widgem_attr(private)]
    items: Vec<GridItem>,
}

impl Grid {
    pub fn item(
        mut self,
        key: impl Into<ChildKey>,
        options: LayoutItemOptions,
        widget: BoxWidget,
    ) -> Self {
        self.items.push(GridItem {
            key: key.into(),
            options,
            widget,
        });
        self
    }
}

impl Widget for Grid {
    fn render(&self, _ctx: WidgetRenderContext) -> BoxWidget {
        todo!()
    }
}
