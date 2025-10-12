use {
    crate::{
        impl_widget_base,
        text::TextHandler,
        widget_initializer::{self, WidgetInitializer},
        Widget, WidgetBaseOf,
    },
    accesskit::Role,
    std::fmt::Display,
};

pub struct Label {
    base: WidgetBaseOf<Self>,
}

impl Label {
    fn new(mut base: WidgetBaseOf<Self>, text: String) -> anyhow::Result<Self> {
        let id = base.id().raw();
        let text_style = base.compute_style();
        base.set_child(0, TextHandler::init(text, text_style))?
            .set_host_id(id);
        Ok(Label { base })
    }

    pub fn init(text: String) -> impl WidgetInitializer<Output = Self> {
        widget_initializer::from_fallible_new_and_set(Self::new, Self::set_text, text)
    }

    #[allow(dead_code)]
    fn text_widget(&self) -> &TextHandler {
        self.base.get_child::<TextHandler>(0).unwrap()
    }

    fn text_widget_mut(&mut self) -> &mut TextHandler {
        self.base.get_child_mut::<TextHandler>(0).unwrap()
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

    fn handle_accessibility_node_request(&mut self) -> anyhow::Result<Option<accesskit::Node>> {
        let mut node = accesskit::Node::new(Role::Label);
        node.set_value(self.text_widget().text().as_str());
        Ok(Some(node))
    }
}
