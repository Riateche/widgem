use {
    super::{Widget, WidgetBaseOf},
    crate::{impl_widget_base, widgets::widget_trait::NewWidget},
};

// TODO: remove
pub struct PaddingBox {
    base: WidgetBaseOf<Self>,
}

impl PaddingBox {
    // TODO: method to set content and options
}

impl NewWidget for PaddingBox {
    type Arg = ();

    fn new(base: WidgetBaseOf<Self>, (): Self::Arg) -> Self {
        Self { base }
    }
    fn handle_declared(&mut self, (): Self::Arg) {}
}

impl Widget for PaddingBox {
    impl_widget_base!();
}
