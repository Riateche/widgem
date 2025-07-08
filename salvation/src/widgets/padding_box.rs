use {
    super::{Widget, WidgetBaseOf},
    crate::impl_widget_common,
};

pub struct PaddingBox {
    common: WidgetBaseOf<Self>,
}

impl PaddingBox {
    // TODO: method to set content and options
}

impl Widget for PaddingBox {
    impl_widget_common!();

    fn new(common: WidgetBaseOf<Self>) -> Self {
        Self { common }
    }
}
