use std::cmp::{max, min};

use crate::{
    layout::SizeHint,
    types::{Point, Rect, Size},
};

use super::{Widget, WidgetCommon};

// TODO: get from style, apply scale
const SPACING: i32 = 10;

pub struct Column {
    // TODO: add layout options
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
            common: WidgetCommon::new(),
        }
    }

    pub fn add(&mut self, widget: Box<dyn Widget>) {
        self.common.add_child(self.common.children.len(), widget);
    }
}

impl Widget for Column {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }
    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }
    fn layout(&mut self) -> Vec<Option<Rect>> {
        let Some(rect_in_window) = self.common().rect_in_window else {
            return Vec::new();
        };
        // TODO: implement shrinking/growing
        let mut current_y = 0;
        let mut new_rects = Vec::new();
        for (i, child) in self.common.children.iter_mut().enumerate() {
            if i != 0 {
                current_y += SPACING;
            }
            let child_size_x = child_size_x(rect_in_window.size.x, child);
            let child_hint_y = child.widget.size_hint_y(child_size_x);
            let child_rect = Rect {
                top_left: Point { x: 0, y: current_y },
                size: Size {
                    x: child_size_x,
                    y: child_hint_y.preferred,
                },
            };
            new_rects.push(Some(child_rect));
            current_y = child_rect.bottom_right().y;
        }
        new_rects
    }
    fn size_hint_x(&mut self) -> SizeHint {
        let mut r = SizeHint {
            min: 0,
            preferred: 0,
            is_fixed: true,
        };
        for child in &mut self.common.children {
            let child_hint = child.widget.size_hint_x();
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
        for (i, child) in self.common.children.iter_mut().enumerate() {
            let child_size_x = child_size_x(size_x, child);
            let child_hint = child.widget.size_hint_y(child_size_x);
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
