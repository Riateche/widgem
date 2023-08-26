use crate::{draw::DrawContext, event::MouseInputEvent, types::Rect, Child, Widget};

#[derive(Default)]
pub struct Stack {
    children: Vec<Child>,
}

impl Stack {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
        }
    }

    pub fn add(&mut self, rect: Rect, widget: impl Widget + 'static) {
        self.children.push(Child {
            rect,
            widget: Box::new(widget),
        });
    }
}

impl Widget for Stack {
    fn draw(&mut self, ctx: &mut DrawContext<'_>) {
        for child in &mut self.children {
            let mut ctx = DrawContext {
                rect: child.rect.translate(ctx.rect.top_left).intersect(ctx.rect),
                pixmap: ctx.pixmap,
                font_system: ctx.font_system,
                font_metrics: ctx.font_metrics,
                swash_cache: ctx.swash_cache,
                palette: ctx.palette,
            };
            child.widget.draw(&mut ctx);
        }
    }

    fn mouse_input(&mut self, event: &mut MouseInputEvent<'_>) {
        for child in &mut self.children {
            if child.rect.contains(event.pos) {
                let mut event = MouseInputEvent {
                    pos: event.pos - child.rect.top_left,

                    device_id: event.device_id,
                    state: event.state,
                    button: event.button,
                    modifiers: event.modifiers,
                    font_metrics: event.font_metrics,
                    palette: event.palette,
                };
                child.widget.mouse_input(&mut event);
            }
        }
    }
}
