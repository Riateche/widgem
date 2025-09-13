use {
    super::{Widget, WidgetBaseOf},
    crate::{impl_widget_base, layout::Layout, widgets::widget_trait::WidgetInitializer},
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

    fn init(self, mut base: WidgetBaseOf<Self::Output>) -> Self::Output {
        base.set_layout(Layout::HorizontalFirst);
        Row { base }
    }

    fn reinit(self, _widget: &mut Self::Output) {}
}

impl Widget for Row {
    impl_widget_base!();
}
