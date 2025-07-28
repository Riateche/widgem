use {
    super::{label::Label, window::Window, Widget, WidgetBaseOf},
    crate::{
        callback::{Callback, Callbacks},
        impl_widget_base,
        shared_window::X11WindowType,
        text_editor::Text,
        widgets::widget_trait::NewWidget,
    },
    winit::window::WindowLevel,
};

pub struct Menu {
    base: WidgetBaseOf<Self>,
}

impl Menu {}

impl NewWidget for Menu {
    type Arg = ();

    fn new(mut base: WidgetBaseOf<Self>, (): Self::Arg) -> Self {
        let window = base
            .add_child::<Window>("Menu".into())
            .set_decorations(false)
            .set_window_level(WindowLevel::AlwaysOnTop)
            .set_x11_window_type(vec![X11WindowType::Menu])
            .set_skip_windows_taskbar(true);
        window
            .base_mut()
            .add_child::<Label>("menu content 1\nmenu content 2\nmenu content 3".into());
        Self { base }
    }
    fn handle_declared(&mut self, (): Self::Arg) {}
}
impl Widget for Menu {
    impl_widget_base!();
}

pub struct MenuItem {
    base: WidgetBaseOf<Self>,
    text: String,
    clicked: Callbacks<()>,
}

impl MenuItem {
    pub fn set_text(&mut self, text: &str) -> &mut Self {
        self.text = text.into();
        self
    }

    pub fn on_clicked(&mut self, callback: Callback<()>) -> &mut Self {
        self.clicked.add(callback);
        self
    }
}

impl NewWidget for MenuItem {
    type Arg = String;

    fn new(base: WidgetBaseOf<Self>, arg: Self::Arg) -> Self {
        Self {
            base,
            text: arg,
            clicked: Default::default(),
        }
    }

    fn handle_declared(&mut self, arg: Self::Arg) {
        self.set_text(&arg);
    }
}

impl Widget for MenuItem {
    impl_widget_base!();

    fn handle_declare_children_request(&mut self) -> anyhow::Result<()> {
        let selector = self.base.style_selector().clone();
        self.base
            .declare_child::<Text>((self.text.clone(), selector))
            .set_multiline(false);
        Ok(())
    }
}
