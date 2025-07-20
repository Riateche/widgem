use {
    super::{Widget, WidgetBaseOf},
    crate::{impl_widget_base, widgets::widget_trait::NewWidget},
};

pub struct Column {
    // TODO: add layout options
    base: WidgetBaseOf<Self>,
}

impl NewWidget for Column {
    type Arg = ();

    fn new(base: WidgetBaseOf<Self>, (): Self::Arg) -> Self {
        Self { base }
    }
    fn handle_declared(&mut self, (): Self::Arg) {}
}

impl Widget for Column {
    impl_widget_base!();
}
