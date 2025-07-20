use {
    super::{Widget, WidgetBaseOf},
    crate::impl_widget_base,
};

pub struct RootWidget {
    base: WidgetBaseOf<Self>,
}

impl RootWidget {
    pub fn new(base: WidgetBaseOf<Self>) -> Self {
        Self { base }
    }
}

impl Widget for RootWidget {
    impl_widget_base!();
}
