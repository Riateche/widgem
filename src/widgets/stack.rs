use crate::{draw::DrawContext, Widget, WidgetContainer, WidgetInfo};

#[derive(Default)]
pub struct Stack {
    children: Vec<WidgetContainer>,
}

impl Stack {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
        }
    }

    pub fn add(&mut self, info: WidgetInfo, widget: impl Widget + 'static) {
        self.children.push(WidgetContainer {
            info,
            widget: Box::new(widget),
        });
    }
}

impl Widget for Stack {
    fn draw(&mut self, ctx: &mut DrawContext<'_>) {
        for child in &mut self.children {
            let mut ctx = DrawContext {
                self_info: &mut child.info,
                pixmap: ctx.pixmap,
                font_system: ctx.font_system,
                font_metrics: ctx.font_metrics,
                swash_cache: ctx.swash_cache,
                palette: ctx.palette,
            };
            child.widget.draw(&mut ctx);
        }
    }
}
