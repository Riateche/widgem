use {
    super::{Widget, WidgetBaseOf, WidgetExt},
    crate::{impl_widget_base, text_editor::Text},
    cosmic_text::Attrs,
    std::fmt::Display,
};

pub struct Label {
    base: WidgetBaseOf<Self>,
}

impl Label {
    #[allow(dead_code)]
    fn text_widget(&self) -> &Text {
        self.base.get_child::<Text>(0).unwrap()
    }

    fn text_widget_mut(&mut self) -> &mut Text {
        self.base.get_child_mut::<Text>(0).unwrap()
    }

    pub fn set_text(&mut self, text: impl Display) -> &mut Self {
        self.text_widget_mut().set_text(text, Attrs::new());
        self.base.size_hint_changed();
        self.base.update();
        self
    }
}

impl Widget for Label {
    impl_widget_base!();

    fn new(mut base: WidgetBaseOf<Self>) -> Self {
        let id = base.id().raw();
        let element = base.style_element().clone();
        base.add_child::<Text>()
            .set_column(0)
            .set_row(0)
            .set_host_id(id)
            .set_host_style_element(element);
        Self { base }
    }
}
