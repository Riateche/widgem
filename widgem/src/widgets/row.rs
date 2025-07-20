use {
    super::{Widget, WidgetBaseOf},
    crate::{impl_widget_base, widgets::widget_trait::NewWidget},
};

// TODO: reimplement auto keys and auto row/column
pub struct Row {
    base: WidgetBaseOf<Self>,
}

impl NewWidget for Row {
    type Arg = ();

    fn new(base: WidgetBaseOf<Self>, (): Self::Arg) -> Self {
        Self { base }
    }
    fn handle_declared(&mut self, (): Self::Arg) {}
}

impl Widget for Row {
    impl_widget_base!();
}
