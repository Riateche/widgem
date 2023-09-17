use std::{
    cmp::{max, min},
    rc::Rc,
};

use crate::{
    draw::DrawEvent,
    event::{
        CursorMovedEvent, GeometryChangedEvent, MountEvent, MouseInputEvent,
        WindowFocusChangedEvent,
    },
    layout::SizeHint,
    types::{Point, Rect, Size},
};

use super::{Geometry, MountPoint, Widget, WidgetCommon, WidgetExt};

// TODO: get from style, apply scale
const SPACING: i32 = 10;

pub struct Child {
    pub rect_in_parent: Rect,
    pub child: super::Child,
}

pub struct Column {
    children: Vec<Child>,
    common: WidgetCommon,
}

fn child_size_x(layout_size_x: i32, child: &mut super::Child) -> i32 {
    let hint = child.widget.size_hint_x();
    if hint.is_fixed {
        min(hint.preferred, layout_size_x)
    } else {
        layout_size_x
    }
}

impl Column {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            common: WidgetCommon::new(),
        }
    }

    pub fn add(&mut self, mut widget: Box<dyn Widget>) {
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
            rect_in_parent: Rect::default(),
            child: super::Child {
                widget,
                index_in_parent,
            },
        });
    }
}

impl Widget for Column {
    fn children_mut(&mut self) -> Box<dyn Iterator<Item = &mut super::Child> + '_> {
        Box::new(self.children.iter_mut().map(|c| &mut c.child))
    }

    fn on_draw(&mut self, event: DrawEvent) {
        for child in &mut self.children {
            let child_event = event.map_to_child(child.rect_in_parent);
            if !child_event.rect().is_empty() {
                child.child.widget.dispatch(child_event.into());
            }
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
                    num_clicks: event.num_clicks,
                    accepted_by: Rc::clone(&event.accepted_by),
                };
                if child.child.widget.dispatch(event.into()) {
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
        let Some(geometry) = self.common().geometry else {
            return;
        };
        // TODO: implement shrinking/growing
        let mut current_y = 0;
        for (i, child) in self.children.iter_mut().enumerate() {
            if i != 0 {
                current_y += SPACING;
            }
            let child_size_x = child_size_x(geometry.rect_in_window.size.x, &mut child.child);
            let child_hint_y = child.child.widget.size_hint_y(child_size_x);
            child.rect_in_parent = Rect {
                top_left: Point { x: 0, y: current_y },
                size: Size {
                    x: child_size_x,
                    y: child_hint_y.preferred,
                },
            };
            current_y = child.rect_in_parent.bottom_right().y;

            let rect = child
                .rect_in_parent
                .translate(geometry.rect_in_window.top_left);
            child.child.widget.dispatch(
                GeometryChangedEvent {
                    new_geometry: Some(Geometry {
                        rect_in_window: rect,
                    }),
                }
                .into(),
            );
            child.child.widget.layout();
        }
    }
    fn size_hint_x(&mut self) -> SizeHint {
        let mut r = SizeHint {
            min: 0,
            preferred: 0,
            is_fixed: true,
        };
        for child in &mut self.children {
            let child_hint = child.child.widget.size_hint_x();
            r.min = max(r.min, child_hint.min);
            r.preferred = max(r.preferred, child_hint.preferred);
            if !child_hint.is_fixed {
                r.is_fixed = false;
            }
        }
        r
    }
    fn size_hint_y(&mut self, size_x: i32) -> SizeHint {
        let mut r = SizeHint {
            min: 0,
            preferred: 0,
            is_fixed: true,
        };
        for (i, child) in self.children.iter_mut().enumerate() {
            let child_size_x = child_size_x(size_x, &mut child.child);
            let child_hint = child.child.widget.size_hint_y(child_size_x);
            if i != 0 {
                r.min += SPACING;
                r.preferred += SPACING;
            }
            r.min += child_hint.min;
            r.preferred += child_hint.preferred;
            if !child_hint.is_fixed {
                r.is_fixed = false;
            }
        }
        r
    }
}
