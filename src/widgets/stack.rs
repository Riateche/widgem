use std::rc::Rc;

use crate::{
    draw::DrawEvent,
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

    fn on_draw(&mut self, event: DrawEvent) -> bool {
        for child in &mut self.children {
            let child_event = DrawEvent {
                rect: child.rect.translate(event.rect.top_left).intersect(event.rect),
                pixmap: Rc::clone(&event.pixmap),
            };
            child.widget.on_draw(child_event);
        }
        true
    }

    fn on_mouse_input(&mut self, event: MouseInputEvent) -> bool {
        for child in &mut self.children {
            if child.rect.contains(event.pos) {
                let event = MouseInputEvent {
                    pos: event.pos - child.rect.top_left,
                    device_id: event.device_id,
                    state: event.state,
                    button: event.button,
                };
                if child.widget.on_mouse_input(event) {
                    return true;
                }
            }
        }
        false
    }

    fn on_cursor_moved(&mut self, event: CursorMovedEvent) -> bool {
        for child in &mut self.children {
            if child.rect.contains(event.pos) {
                let event = CursorMovedEvent {
                    pos: event.pos - child.rect.top_left,
                    device_id: event.device_id,
                };
                if child.widget.on_cursor_moved(event) {
                    return true;
                }
            }
        }
        false
    }

    fn common(&self) -> &WidgetCommon {
        &self.common
    }
    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }
}
