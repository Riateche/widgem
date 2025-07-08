use {
    super::{Widget, WidgetBaseOf},
    crate::impl_widget_common,
};

// TODO: reimplement auto keys and auto row/column
pub struct Row {
    common: WidgetBaseOf<Self>,
}

impl Widget for Row {
    impl_widget_common!();

    fn new(common: WidgetBaseOf<Self>) -> Self {
        Self { common }
    }
}
