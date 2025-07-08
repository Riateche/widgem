use {
    super::{Widget, WidgetBaseOf},
    crate::impl_widget_common,
};

pub struct RootWidget {
    common: WidgetBaseOf<Self>,
}

impl Widget for RootWidget {
    impl_widget_common!();

    fn new(common: WidgetBaseOf<Self>) -> Self {
        Self { common }
    }
}
