use {
    super::{Widget, WidgetCommon, WidgetCommonTyped},
    crate::impl_widget_common,
};

pub struct PaddingBox {
    common: WidgetCommon,
}

impl PaddingBox {
    // TODO: method to set content and options
}

impl Widget for PaddingBox {
    impl_widget_common!();

    fn new(common: WidgetCommonTyped<Self>) -> Self {
        Self {
            common: common.into(),
        }
    }
}
