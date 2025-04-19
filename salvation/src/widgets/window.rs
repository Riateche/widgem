use crate::impl_widget_common;

use super::{Widget, WidgetCommon, WidgetCommonTyped};

pub struct WindowWidget {
    common: WidgetCommon,
}

impl Widget for WindowWidget {
    impl_widget_common!();

    fn is_window_root_type() -> bool {
        true
    }

    fn new(common: WidgetCommonTyped<Self>) -> Self {
        Self {
            common: common.into(),
        }
    }
}
