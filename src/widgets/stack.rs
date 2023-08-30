use crate::{
    draw::DrawContext,
    event::{CursorMovedEvent, MouseInputEvent},
    types::Rect,
};

use super::{mount, Child, Widget, WidgetCommon};

pub struct Stack {
    children: Vec<Child>,
    common: WidgetCommon,
}

impl Stack {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            common: WidgetCommon::new(),
        }
    }

    pub fn add(&mut self, rect: Rect, mut widget: Box<dyn Widget>) {
        if let (Some(system), Some(address)) = (&self.common.system, &self.common.address) {
            mount(widget.as_mut(), system.clone(), address.clone());
        }
        self.children.push(Child { rect, widget });
    }
}

impl Widget for Stack {
    fn children_mut(&mut self) -> Box<dyn Iterator<Item = &mut Box<dyn Widget>> + '_> {
        Box::new(self.children.iter_mut().map(|c| &mut c.widget))
    }

    fn draw(&mut self, ctx: &mut DrawContext<'_>) {
        for child in &mut self.children {
            let mut ctx = DrawContext {
                rect: child.rect.translate(ctx.rect.top_left).intersect(ctx.rect),
                pixmap: ctx.pixmap,
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
                    pressed_mouse_buttons: event.pressed_mouse_buttons,
                };
                child.widget.mouse_input(&mut event);
            }
        }
    }

    fn cursor_moved(&mut self, event: &mut CursorMovedEvent<'_>) {
        for child in &mut self.children {
            if child.rect.contains(event.pos) {
                let mut event = CursorMovedEvent {
                    pos: event.pos - child.rect.top_left,
                    device_id: event.device_id,
                    modifiers: event.modifiers,
                    pressed_mouse_buttons: event.pressed_mouse_buttons,
                };
                child.widget.cursor_moved(&mut event);
            }
        }
    }

    fn common(&self) -> &WidgetCommon {
        &self.common
    }
    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }
}
