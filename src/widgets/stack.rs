use crate::{
    event::{GeometryChangedEvent, MountEvent},
    layout::SizeHint,
    types::Rect,
};

use super::{Child, MountPoint, Widget, WidgetCommon, WidgetExt};

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
            widget,
            index_in_parent,
            rect_in_parent: Some(rect),
        });
    }
}

impl Widget for Stack {
    fn children_mut(&mut self) -> Box<dyn Iterator<Item = &mut super::Child> + '_> {
        Box::new(self.children.iter_mut())
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
            if let Some(rect_in_parent) = child.rect_in_parent {
                let rect = rect_in_parent.translate(self_rect.top_left);
                child.widget.dispatch(
                    GeometryChangedEvent {
                        new_rect_in_window: Some(rect),
                    }
                    .into(),
                );
            }
        }
    }

    fn size_hint_x(&mut self) -> SizeHint {
        let max = self
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
