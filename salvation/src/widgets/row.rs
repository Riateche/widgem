use {
    super::{Widget, WidgetCommon, WidgetCommonTyped},
    crate::impl_widget_common,
};

// TODO: reimplement auto keys and auto row/column
pub struct Row {
    common: WidgetCommon,
}

impl Widget for Row {
    impl_widget_common!();

    fn new(common: WidgetCommonTyped<Self>) -> Self {
        Self {
            common: common.into(),
        }
    }
}
