use {
    super::{Widget, WidgetBaseOf},
    crate::impl_widget_common,
};

pub struct Column {
    // TODO: add layout options
    common: WidgetBaseOf<Self>,
}

impl Widget for Column {
    impl_widget_common!();

    fn new(common: WidgetBaseOf<Self>) -> Self {
        Self { common }
    }
}
