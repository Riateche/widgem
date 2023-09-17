use std::{cmp::max, rc::Rc};

use crate::{
    draw::DrawEvent,
    event::{CursorMovedEvent, GeometryChangedEvent, MouseInputEvent, WindowFocusChangedEvent},
    layout::SizeHint,
    types::{Point, Rect},
};

use super::{Child, Geometry, Widget, WidgetCommon, WidgetExt};

const PADDING: Point = Point { x: 10, y: 10 };

#[derive(Default)]
pub struct PaddingBox {
    content: Option<Child>,
    content_rect_in_parent: Rect,
    common: WidgetCommon,
}

impl PaddingBox {
    pub fn new(content: Box<dyn Widget>) -> Self {
        Self {
            content: Some(Child {
                widget: content,
                index_in_parent: 0,
            }),
            content_rect_in_parent: Rect::default(),
            common: WidgetCommon::new(),
        }
    }
    // TODO: method to set content
}

impl Widget for PaddingBox {
    fn common(&self) -> &super::WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut super::WidgetCommon {
        &mut self.common
    }

    fn children_mut(&mut self) -> Box<dyn Iterator<Item = &mut super::Child> + '_> {
        Box::new(self.content.as_mut().into_iter())
    }

    fn on_draw(&mut self, event: DrawEvent) {
        if let Some(content) = &mut self.content {
            let child_event = event.map_to_child(self.content_rect_in_parent);
            if !child_event.rect().is_empty() {
                content.widget.dispatch(child_event.into());
            }
        }
    }

    fn on_mouse_input(&mut self, event: MouseInputEvent) -> bool {
        if let Some(content) = &mut self.content {
            if self.content_rect_in_parent.contains(event.pos) {
                let event = MouseInputEvent {
                    pos: event.pos - self.content_rect_in_parent.top_left,
                    device_id: event.device_id,
                    state: event.state,
                    button: event.button,
                    num_clicks: event.num_clicks,
                    accepted_by: Rc::clone(&event.accepted_by),
                };
                if content.widget.dispatch(event.into()) {
                    return true;
                }
            }
        }
        false
    }

    fn on_cursor_moved(&mut self, event: CursorMovedEvent) -> bool {
        if let Some(content) = &mut self.content {
            if self.content_rect_in_parent.contains(event.pos) {
                let event = CursorMovedEvent {
                    pos: event.pos - self.content_rect_in_parent.top_left,
                    device_id: event.device_id,
                    accepted_by: event.accepted_by.clone(),
                };
                if content.widget.dispatch(event.into()) {
                    return true;
                }
            }
        }
        false
    }

    fn on_window_focus_changed(&mut self, event: WindowFocusChangedEvent) {
        if let Some(content) = &mut self.content {
            content.widget.dispatch(event.clone().into());
        }
    }

    fn size_hint_x(&mut self) -> SizeHint {
        let mut size_hint = if let Some(content) = &mut self.content {
            content.widget.size_hint_x()
        } else {
            SizeHint {
                min: 0,
                preferred: 0,
                is_fixed: true,
            }
        };
        size_hint.min += PADDING.x * 2;
        size_hint.preferred += PADDING.x * 2;
        size_hint
    }

    fn size_hint_y(&mut self, size_x: i32) -> SizeHint {
        let child_size_x = max(0, size_x - 2 * PADDING.x);
        let mut size_hint = if let Some(content) = &mut self.content {
            content.widget.size_hint_y(child_size_x)
        } else {
            SizeHint {
                min: 0,
                preferred: 0,
                is_fixed: true,
            }
        };
        size_hint.min += PADDING.y * 2;
        size_hint.preferred += PADDING.y * 2;
        size_hint
    }

    fn layout(&mut self) {
        let Some(geometry) = self.common.geometry else {
            return;
        };
        let mut rect = geometry.rect_in_window;
        rect.top_left.x += PADDING.x;
        rect.top_left.y += PADDING.y;
        rect.size.x = max(0, rect.size.x - 2 * PADDING.x);
        rect.size.y = max(0, rect.size.y - 2 * PADDING.y);
        self.content_rect_in_parent = rect;
        if let Some(content) = &mut self.content {
            content.widget.dispatch(
                GeometryChangedEvent {
                    new_geometry: Some(Geometry {
                        rect_in_window: rect,
                    }),
                }
                .into(),
            );
            content.widget.layout();
        }
    }
}
