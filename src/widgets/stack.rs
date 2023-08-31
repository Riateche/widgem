use crate::{
    draw::DrawContext,
    event::{CursorMovedEvent, MouseInputEvent},
    types::Rect,
};

use super::{mount, Child, MountPoint, Widget, WidgetCommon};

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
        if let Some(mount_point) = &self.common.mount_point {
            let address = mount_point.address.clone().join(widget.common().id);
            mount(
                widget.as_mut(),
                MountPoint {
                    address,
                    system: mount_point.system.clone(),
                    window: mount_point.window.clone(),
                },
            );
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

    fn mouse_input(&mut self, event: &mut MouseInputEvent) {
        for child in &mut self.children {
            if child.rect.contains(event.pos) {
                let mut event = MouseInputEvent {
                    pos: event.pos - child.rect.top_left,
                    device_id: event.device_id,
                    state: event.state,
                    button: event.button,
                };
                child.widget.mouse_input(&mut event);
            }
        }
    }

    fn cursor_moved(&mut self, event: &mut CursorMovedEvent) {
        for child in &mut self.children {
            if child.rect.contains(event.pos) {
                let mut event = CursorMovedEvent {
                    pos: event.pos - child.rect.top_left,
                    device_id: event.device_id,
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
