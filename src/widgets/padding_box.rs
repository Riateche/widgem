use anyhow::Result;
use std::cmp::max;

use crate::{
    layout::SizeHint,
    types::{Point, Rect, Size},
};

use super::{Widget, WidgetCommon, WidgetExt};

const PADDING: Point = Point { x: 10, y: 10 };

#[derive(Default)]
pub struct PaddingBox {
    common: WidgetCommon,
}

impl PaddingBox {
    pub fn new(content: Box<dyn Widget>) -> Self {
        let mut common = WidgetCommon::new();
        common.add_child(0, content);
        Self { common }
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

    fn size_hint_x(&mut self) -> Result<SizeHint> {
        let mut size_hint = if let Some(content) = self.common.children.get_mut(0) {
            content.widget.cached_size_hint_x()
        } else {
            SizeHint {
                min: 0,
                preferred: 0,
                is_fixed: true,
            }
        };
        size_hint.min += PADDING.x * 2;
        size_hint.preferred += PADDING.x * 2;
        Ok(size_hint)
    }

    fn size_hint_y(&mut self, size_x: i32) -> Result<SizeHint> {
        let child_size_x = max(0, size_x - 2 * PADDING.x);
        let mut size_hint = if let Some(content) = self.common.children.get_mut(0) {
            content.widget.cached_size_hint_y(child_size_x)
        } else {
            SizeHint {
                min: 0,
                preferred: 0,
                is_fixed: true,
            }
        };
        size_hint.min += PADDING.y * 2;
        size_hint.preferred += PADDING.y * 2;
        Ok(size_hint)
    }

    fn layout(&mut self) -> Result<Vec<Option<Rect>>> {
        if self.common.children.is_empty() {
            return Ok(Vec::new());
        }

        let self_rect = self.common.rect_in_window_or_err()?;
        let rect = Rect {
            top_left: PADDING,
            size: Size {
                x: max(0, self_rect.size.x - 2 * PADDING.x),
                y: max(0, self_rect.size.y - 2 * PADDING.y),
            },
        };
        Ok(vec![Some(rect)])
    }
}
