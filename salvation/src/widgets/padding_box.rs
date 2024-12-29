use {
    super::{Widget, WidgetCommon},
    crate::{impl_widget_common, layout::LayoutItemOptions},
};

pub struct PaddingBox {
    common: WidgetCommon,
}

impl PaddingBox {
    pub fn new(content: Box<dyn Widget>) -> Self {
        let mut common = WidgetCommon::new::<Self>();
        common.add_child(content, LayoutItemOptions::from_pos_in_grid(0, 0));
        Self {
            common: common.into(),
        }
    }
    // TODO: method to set content and options
}

impl Widget for PaddingBox {
    impl_widget_common!();
}
