use {
    super::{Widget, WidgetBaseOf},
    crate::{
        impl_widget_base,
        layout::Layout,
        widgets::widget_trait::{NewWidget, WidgetInitializer},
    },
};

// TODO: reimplement auto keys and auto row/column
pub struct Row {
    base: WidgetBaseOf<Self>,
}

impl Row {
    pub fn init() -> impl WidgetInitializer<Output = Self> {
        Initializer
    }
}

struct Initializer;

impl WidgetInitializer for Initializer {
    type Output = Row;

    fn init(self, base: WidgetBaseOf<Self::Output>) -> Self::Output {
        Row { base }
    }

    fn reinit(self, _widget: &mut Self::Output) {}
}

impl NewWidget for Row {
    type Arg = ();

    fn new(mut base: WidgetBaseOf<Self>, (): Self::Arg) -> Self {
        base.set_layout(Layout::HorizontalFirst);
        Self { base }
    }
    fn handle_declared(&mut self, (): Self::Arg) {}
}

impl Widget for Row {
    impl_widget_base!();
}
