use {
    crate::{
        impl_widget_base,
        text_editor::Text,
        widget_initializer::{self, WidgetInitializer},
        Widget, WidgetBaseOf,
    },
    std::fmt::Display,
};

pub struct Label {
    base: WidgetBaseOf<Self>,
}

impl Label {
    fn new(mut base: WidgetBaseOf<Self>, text: String) -> anyhow::Result<Self> {
        let id = base.id().raw();
        let text_style = base.compute_style();
        base.set_child(0, Text::init(text, text_style))?
            .set_host_id(id);
        Ok(Label { base })
    }

    pub fn init(text: String) -> impl WidgetInitializer<Output = Self> {
        widget_initializer::from_fallible_new_and_set(Self::new, Self::set_text, text)
    }

    #[allow(dead_code)]
    fn text_widget(&self) -> &Text {
        self.base.get_child::<Text>(0).unwrap()
    }

    fn text_widget_mut(&mut self) -> &mut Text {
        self.base.get_child_mut::<Text>(0).unwrap()
    }

    pub fn set_text(&mut self, text: impl Display) -> &mut Self {
        self.text_widget_mut().set_text(text);
        self.base.size_hint_changed();
        self.base.update();
        self
    }
}

impl Widget for Label {
    impl_widget_base!();
}
