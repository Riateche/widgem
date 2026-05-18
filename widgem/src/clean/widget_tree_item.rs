use {
    crate::{
        clean::{BoxWidget, WidgetContext},
        ChildKey,
    },
    std::collections::BTreeMap,
};

pub struct WidgetTreeItem {
    context: WidgetContext,
    widget: BoxWidget,
    children: BTreeMap<ChildKey, WidgetTreeItem>,
}

impl WidgetTreeItem {
    pub fn new(widget: BoxWidget, context: WidgetContext) -> Self {
        Self {
            widget,
            context,
            children: BTreeMap::new(),
        }
    }

    pub fn widget(&self) -> &BoxWidget {
        &self.widget
    }

    pub fn widget_mut(&mut self) -> &mut BoxWidget {
        &mut self.widget
    }

    pub fn context(&self) -> &WidgetContext {
        &self.context
    }

    pub fn render(&self) -> BoxWidget {
        self.widget.render(super::WidgetRenderContext {
            widget_context: self.context.clone(),
        })
    }
}
