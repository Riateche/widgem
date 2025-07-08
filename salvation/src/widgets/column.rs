use {
    super::{Widget, WidgetBaseOf},
    crate::impl_widget_base,
};

pub struct Column {
    // TODO: add layout options
    base: WidgetBaseOf<Self>,
}

impl Widget for Column {
    impl_widget_base!();

    fn new(base: WidgetBaseOf<Self>) -> Self {
        Self { base }
    }
}
