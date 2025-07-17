use {
    super::{Widget, WidgetBaseOf},
    crate::impl_widget_base,
};

// TODO: remove
pub struct PaddingBox {
    base: WidgetBaseOf<Self>,
}

impl PaddingBox {
    // TODO: method to set content and options
}

impl Widget for PaddingBox {
    impl_widget_base!();

    fn new(base: WidgetBaseOf<Self>) -> Self {
        Self { base }
    }
}
