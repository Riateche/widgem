use {
    super::{label::Label, window::WindowWidget, Widget, WidgetBaseOf, WidgetExt},
    crate::{impl_widget_base, window::X11WindowType},
    winit::window::WindowLevel,
};

pub struct Menu {
    base: WidgetBaseOf<Self>,
}

impl Menu {}

impl Widget for Menu {
    impl_widget_base!();

    fn new(mut base: WidgetBaseOf<Self>) -> Self {
        let window = base
            .add_child::<WindowWidget>()
            .set_decorations(false)
            .set_window_level(WindowLevel::AlwaysOnTop)
            .set_x11_window_type(vec![X11WindowType::Menu])
            .set_skip_windows_taskbar(true);
        window
            .base_mut()
            .add_child::<Label>()
            .set_column(0)
            .set_row(0)
            .set_text("menu content 1\nmenu content 2\nmenu content 3");
        Self { base }
    }
}
