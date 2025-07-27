use {
    super::{Widget, WidgetBaseOf},
    crate::{impl_widget_base, text_editor::Text, widgets::widget_trait::NewWidget},
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

impl NewWidget for Label {
    type Arg = String;

    fn new(mut base: WidgetBaseOf<Self>, arg: Self::Arg) -> Self {
        let id = base.id().raw();
        let element = base.style_selector().clone();
        base.add_child::<Text>(arg)
            .set_host_id(id)
            .set_host_style_selector(element);
        Self { base }
    }

    fn handle_declared(&mut self, arg: Self::Arg) {
        self.set_text(arg);
    }
}

impl Widget for Label {
    impl_widget_base!();
}
