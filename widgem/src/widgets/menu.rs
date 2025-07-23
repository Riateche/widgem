use {
    super::{label::Label, window::Window, Widget, WidgetBaseOf},
    crate::{impl_widget_base, shared_window::X11WindowType, widgets::widget_trait::NewWidget},
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
