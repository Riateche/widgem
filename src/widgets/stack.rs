use std::rc::Rc;

use crate::{
    draw::DrawEvent,
    event::{
        CursorMovedEvent, GeometryChangedEvent, MountEvent, MouseInputEvent,
        WindowFocusChangedEvent,
    },
    types::Rect,
};

use super::{Child, Geometry, MountPoint, Widget, WidgetCommon, WidgetExt};

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
            widget.dispatch(
                MountEvent(MountPoint {
                    address,
                    window: mount_point.window.clone(),
                })
                .into(),
            );
        }
        self.children.push(Child {
            rect_in_parent: rect,
            widget,
        });
    }
}

impl Widget for Stack {
    fn children_mut(&mut self) -> Box<dyn Iterator<Item = &mut Box<dyn Widget>> + '_> {
        Box::new(self.children.iter_mut().map(|c| &mut c.widget))
    }

    fn on_draw(&mut self, event: DrawEvent) {
        for child in &mut self.children {
            let child_event = DrawEvent {
                rect: child
                    .rect_in_parent
                    .translate(event.rect.top_left)
                    .intersect(event.rect),
                pixmap: Rc::clone(&event.pixmap),
            };
            child.widget.dispatch(child_event.into());
        }
    }

    fn on_mouse_input(&mut self, event: MouseInputEvent) -> bool {
        for child in &mut self.children {
            if child.rect_in_parent.contains(event.pos) {
                let event = MouseInputEvent {
                    pos: event.pos - child.rect_in_parent.top_left,
                    device_id: event.device_id,
                    state: event.state,
                    button: event.button,
                    accepted_by: Rc::clone(&event.accepted_by),
                };
                if child.widget.dispatch(event.into()) {
                    return true;
                }
            }
        }
        false
    }

    fn on_cursor_moved(&mut self, event: CursorMovedEvent) -> bool {
        for child in &mut self.children {
            if child.rect_in_parent.contains(event.pos) {
                let event = CursorMovedEvent {
                    pos: event.pos - child.rect_in_parent.top_left,
                    device_id: event.device_id,
                };
                if child.widget.dispatch(event.into()) {
                    return true;
                }
            }
        }
        false
    }

    fn on_window_focus_changed(&mut self, event: WindowFocusChangedEvent) {
        for child in &mut self.children {
            child.widget.dispatch(event.clone().into());
        }
    }

    fn common(&self) -> &WidgetCommon {
        &self.common
    }
    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }
    fn layout(&mut self) {
        let Some(geometry) = self.common().geometry else {
            return;
        };
        for child in &mut self.children {
            let rect = child
                .rect_in_parent
                .translate(geometry.rect_in_window.top_left);
            child.widget.dispatch(
                GeometryChangedEvent {
                    new_geometry: Some(Geometry {
                        rect_in_window: rect,
                    }),
                }
                .into(),
            );
        }
    }
}
