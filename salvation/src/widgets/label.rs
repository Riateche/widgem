use {
    super::{Widget, WidgetCommon, WidgetCommonTyped},
    crate::{impl_widget_common, layout::LayoutItemOptions, text_editor::Text},
    cosmic_text::Attrs,
    std::fmt::Display,
};

pub struct Label {
    common: WidgetCommon,
}

impl Label {
    #[allow(dead_code)]
    fn text_widget(&self) -> &Text {
        self.common.children[0]
            .widget
            .downcast_ref::<Text>()
            .unwrap()
    }

    fn text_widget_mut(&mut self) -> &mut Text {
        self.common.children[0]
            .widget
            .downcast_mut::<Text>()
            .unwrap()
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
        common.add_child::<Text>(LayoutItemOptions::from_pos_in_grid(0, 0));
        common.set_no_padding(true);
        Self {
            common: common.into(),
        }
    }
}
