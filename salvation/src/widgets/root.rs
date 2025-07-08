use {
    super::{Widget, WidgetBaseOf},
    crate::impl_widget_base,
};

pub struct RootWidget {
    base: WidgetBaseOf<Self>,
}

impl Widget for RootWidget {
    impl_widget_base!();

    fn new(base: WidgetBaseOf<Self>) -> Self {
        Self { base }
    }
}
