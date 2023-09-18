use std::cmp::max;

use crate::{event::GeometryChangeEvent, layout::SizeHint, types::Point};

use super::{Child, Widget, WidgetCommon, WidgetExt};

const PADDING: Point = Point { x: 10, y: 10 };

#[derive(Default)]
pub struct PaddingBox {
    content: Option<Child>,
    common: WidgetCommon,
}

impl PaddingBox {
    pub fn new(content: Box<dyn Widget>) -> Self {
        Self {
            content: Some(Child {
                widget: content,
                index_in_parent: 0,
                rect_in_parent: None,
            }),
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
        if let Some(content) = &mut self.content {
            let Some(self_rect) = self.common.rect_in_window else {
                content.rect_in_parent = None;
                return;
            };
            let mut rect = self_rect;
            rect.top_left.x += PADDING.x;
            rect.top_left.y += PADDING.y;
            rect.size.x = max(0, rect.size.x - 2 * PADDING.x);
            rect.size.y = max(0, rect.size.y - 2 * PADDING.y);
            content.rect_in_parent = Some(rect);
            content.widget.dispatch(
                GeometryChangeEvent {
                    new_rect_in_window: Some(rect),
                }
                .into(),
            );
            content.widget.layout();
        }
    }
}
