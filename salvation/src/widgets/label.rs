use {
    super::{Widget, WidgetCommon, WidgetCommonTyped, WidgetExt},
    crate::{impl_widget_common, text_editor::Text},
    cosmic_text::Attrs,
    std::fmt::Display,
};

pub struct Label {
    common: WidgetCommonTyped<Self>,
}

impl Label {
    #[allow(dead_code)]
    fn text_widget(&self) -> &Text {
        self.common.get_child::<Text>(0).unwrap()
    }

    fn text_widget_mut(&mut self) -> &mut Text {
        self.common.get_child_mut::<Text>(0).unwrap()
    }

    pub fn set_text(&mut self, text: impl Display) -> &mut Self {
        self.text_widget_mut().set_text(text, Attrs::new());
        self.common.size_hint_changed();
        self.common.update();
        self
    }
}

impl Widget for Label {
    impl_widget_common!();

    fn new(mut common: WidgetCommonTyped<Self>) -> Self {
        common.add_child::<Text>(0).set_column(0).set_row(0);
        common.set_no_padding(true);
        Self {
            common,
        }
    }
}
