use crate::{
    draw::DrawEvent,
    event::{
        CursorMovedEvent, GeometryChangedEvent, MountEvent, MouseInputEvent,
        WindowFocusChangedEvent,
    },
    layout::SizeHint,
    types::Rect,
};

use super::{MountPoint, Widget, WidgetCommon, WidgetExt};

pub struct Child {
    pub rect_in_parent: Rect,
    pub child: super::Child,
}

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
        let index_in_parent = self.children.len() as i32;
        if let Some(mount_point) = &self.common.mount_point {
            let address = mount_point.address.clone().join(widget.common().id);
            widget.dispatch(
                MountEvent(MountPoint {
                    address,
                    window: mount_point.window.clone(),
                    index_in_parent,
                })
                .into(),
            );
        }
        self.children.push(Child {
            rect_in_parent: rect,
            child: super::Child {
                widget,
                index_in_parent,
            },
        });
    }
}

impl Widget for Stack {
    fn children_mut(&mut self) -> Box<dyn Iterator<Item = &mut super::Child> + '_> {
        Box::new(self.children.iter_mut().map(|c| &mut c.child))
    }

    fn on_draw(&mut self, event: DrawEvent) {
        for child in &mut self.children {
            let child_event = event.map_to_child(child.rect_in_parent);
            child.child.widget.dispatch(child_event.into());
        }
    }

    fn on_mouse_input(&mut self, event: MouseInputEvent) -> bool {
        for child in &mut self.children {
            if let Some(child_event) = event.map_to_child(child.rect_in_parent) {
                if child.child.widget.dispatch(child_event.into()) {
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
                    accepted_by: event.accepted_by.clone(),
                };
                if child.child.widget.dispatch(event.into()) {
                    return true;
                }
            }
        }
        false
    }

    fn on_window_focus_changed(&mut self, event: WindowFocusChangedEvent) {
        for child in &mut self.children {
            child.child.widget.dispatch(event.clone().into());
        }
    }

    fn common(&self) -> &WidgetCommon {
        &self.common
    }
    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }
    fn layout(&mut self) {
        let Some(self_rect) = self.common().rect_in_window else {
            return;
        };
        for child in &mut self.children {
            let rect = child.rect_in_parent.translate(self_rect.top_left);
            child.child.widget.dispatch(
                GeometryChangedEvent {
                    new_rect_in_window: Some(rect),
                }
                .into(),
            );
        }
    }

    fn size_hint_x(&mut self) -> SizeHint {
        let max = self
            .children
            .iter()
            .map(|c| c.rect_in_parent.bottom_right().x)
            .max()
            .unwrap_or(0);
        SizeHint {
            min: max,
            preferred: max,
            is_fixed: true,
        }
    }

    fn size_hint_y(&mut self, _size_x: i32) -> SizeHint {
        let max = self
            .children
            .iter()
            .map(|c| c.rect_in_parent.bottom_right().y)
            .max()
            .unwrap_or(0);
        SizeHint {
            min: max,
            preferred: max,
            is_fixed: true,
        }
    }
}
