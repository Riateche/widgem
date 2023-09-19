use std::collections::HashMap;

use crate::{layout::SizeHint, types::Rect};

use super::{RawWidgetId, Widget, WidgetCommon};

pub struct Stack {
    common: WidgetCommon,
    rects: HashMap<RawWidgetId, Option<Rect>>,
}

impl Stack {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            common: WidgetCommon::new(),
            rects: HashMap::new(),
        }
    }

    pub fn add(&mut self, rect: Rect, widget: Box<dyn Widget>) {
        let index = self.common.children.len();
        let id = widget.common().id;
        self.common.add_child(index, widget);
        self.rects.insert(id, Some(rect));
    }
}

impl Widget for Stack {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }
    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }
    fn layout(&mut self) -> Vec<Option<Rect>> {
        let mut new_rects = Vec::new();
        for child in &mut self.common.children {
            new_rects.push(self.rects.get(&child.widget.common().id).copied().flatten());
        }
        new_rects
    }

    fn size_hint_x(&mut self) -> SizeHint {
        let max = self
            .common
            .children
            .iter()
            .filter_map(|c| c.rect_in_parent)
            .map(|rect| rect.bottom_right().x)
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
            .common
            .children
            .iter()
            .filter_map(|c| c.rect_in_parent)
            .map(|rect| rect.bottom_right().y)
            .max()
            .unwrap_or(0);
        SizeHint {
            min: max,
            preferred: max,
            is_fixed: true,
        }
    }
}
