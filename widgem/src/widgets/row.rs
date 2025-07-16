use {
    super::{Widget, WidgetBaseOf},
    crate::impl_widget_base,
};

// TODO: reimplement auto keys and auto row/column
pub struct Row {
    base: WidgetBaseOf<Self>,
}

impl Widget for Row {
    impl_widget_base!();

    fn new(base: WidgetBaseOf<Self>) -> Self {
        Self { base }
    }
}
