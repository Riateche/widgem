use {
    super::{Widget, WidgetCommon, WidgetExt},
    crate::{impl_widget_common, layout::LayoutItemOptions, text_editor::Text},
    cosmic_text::Attrs,
    std::fmt::Display,
};

pub struct Label {
    common: WidgetCommon,
}

impl Label {
    pub fn new(text: impl Display) -> Self {
        let mut common = WidgetCommon::new::<Self>();
        let editor = Text::new(text);
        common.add_child(editor.boxed(), LayoutItemOptions::from_pos_in_grid(0, 0));
        common.set_no_padding(true);
        Self {
            common: common.into(),
        }
    }

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

    pub fn set_text(&mut self, text: impl Display) {
        self.text_widget_mut().set_text(text, Attrs::new());
        self.common.size_hint_changed();
        self.common.update();
    }
}

impl Widget for Label {
    impl_widget_common!();
}
