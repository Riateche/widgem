use {
    crate::{
        event::{FocusReason, MouseInputEvent},
        impl_widget_base,
        text::TextHandler,
        widget_initializer::{self, WidgetInitializer},
        RawWidgetId, Widget, WidgetBaseOf,
    },
    accesskit::Role,
    std::fmt::Display,
    winit::event::{ElementState, MouseButton},
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

    pub fn set_target(&mut self, target_id: RawWidgetId) -> &mut Self {
        let Some(window) = self.base.window() else {
            return self;
        };
        let id = self.base.id().raw();
        if let Some(old_target_id) = window.label_to_target(id) {
            if old_target_id != target_id {
                window.remove_label_link(id, old_target_id);
                window.add_label_link(id, target_id);
                self.base.update(); // TODO: update other widget?
            }
        } else {
            window.add_label_link(id, target_id);
            self.base.update();
        }
        self
    }

    pub fn target(&self) -> Option<RawWidgetId> {
        let id = self.base.id().raw();
        self.base.window()?.label_to_target(id)
    }
}

impl Widget for Label {
    impl_widget_base!();

    fn handle_accessibility_node_request(&mut self) -> anyhow::Result<Option<accesskit::Node>> {
        let mut node = accesskit::Node::new(Role::Label);
        node.set_value(self.text_widget().text().as_str());
        if self.target().is_some() {
            node.set_hidden();
        }
        Ok(Some(node))
    }

    fn handle_mouse_input(&mut self, event: MouseInputEvent) -> anyhow::Result<bool> {
        if event.button == MouseButton::Left && event.state == ElementState::Pressed {
            let window = self.base.window_or_err()?;
            let id = self.base.id().raw();
            if let Some(target_id) = window.label_to_target(id) {
                self.base
                    .app()
                    .set_focus(window.id(), target_id, FocusReason::Mouse);
                return Ok(true);
            }
        }
        Ok(false)
    }
}

impl Drop for Label {
    fn drop(&mut self) {
        if let Some(window) = self.base.window() {
            let id = self.base.id().raw();
            if let Some(old_target_id) = window.label_to_target(id) {
                window.remove_label_link(id, old_target_id);
            }
        }
    }
}
