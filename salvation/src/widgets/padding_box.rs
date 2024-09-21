use crate::layout::LayoutItemOptions;

use super::{Widget, WidgetCommon};

#[derive(Default)]
pub struct PaddingBox {
    common: WidgetCommon,
}

impl PaddingBox {
    pub fn new(content: Box<dyn Widget>) -> Self {
        let mut common = WidgetCommon::new();
        common.add_child(content, LayoutItemOptions::from_pos_in_grid(0, 0));
        Self { common }
    }
    // TODO: method to set content and options
}

impl Widget for PaddingBox {
    fn common(&self) -> &super::WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut super::WidgetCommon {
        &mut self.common
    }
}
