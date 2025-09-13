use {
    super::{Widget, WidgetBaseOf},
    crate::{
        impl_widget_base,
        text_editor::Text,
        widgets::widget_trait::{NewWidget, WidgetInitializer},
    },
    cosmic_text::Attrs,
    std::fmt::Display,
};

pub struct Label {
    base: WidgetBaseOf<Self>,
}

impl Label {
    pub fn init(text: String) -> impl WidgetInitializer<Output = Self> {
        Initializer { text }
    }

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

struct Initializer {
    text: String,
}

impl WidgetInitializer for Initializer {
    type Output = Label;

    fn init(self, mut base: WidgetBaseOf<Self::Output>) -> Self::Output {
        let id = base.id().raw();
        let text_style = base.compute_style();
        base.add_child(Text::init(self.text, text_style))
            .set_host_id(id);
        Label { base }
    }

    fn reinit(self, widget: &mut Self::Output) {
        widget.set_text(self.text);
    }
}

impl NewWidget for Label {
    type Arg = String;

    fn new(mut base: WidgetBaseOf<Self>, arg: Self::Arg) -> Self {
        let id = base.id().raw();
        let text_style = base.compute_style();
        base.add_child(Text::init(arg, text_style)).set_host_id(id);
        Self { base }
    }

    fn handle_declared(&mut self, arg: Self::Arg) {
        self.set_text(arg);
    }
}

impl Widget for Label {
    impl_widget_base!();
}
/*

    pub fn init(text: String) -> impl WidgetInitializer<Output = Self> {
        Initializer { text }
    }

struct Initializer {
    text: String,
}

impl WidgetInitializer for Initializer {
    type Output = Label;

    fn init(self, mut base: WidgetBaseOf<Self::Output>) -> Self::Output {

    }

    fn reinit(self, widget: &mut Self::Output) {
        widget.set_text(self.text);
    }
}

*/
